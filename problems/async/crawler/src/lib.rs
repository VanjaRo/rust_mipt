#![forbid(unsafe_code)]

use futures::future::select_all;
use linkify::{LinkFinder, LinkKind};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{channel, unbounded_channel, Receiver, UnboundedSender};

#[derive(Clone, Default)]
pub struct Config {
    pub concurrent_requests: Option<usize>,
}

pub struct Page {
    pub url: String,
    pub body: String,
}

pub struct Crawler {
    config: Config,
}

impl Crawler {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn run(&mut self, site: String) -> Receiver<Page> {
        let rate_limit = self.config.concurrent_requests.unwrap_or(1);

        let (page_snd, page_rcv) = channel(rate_limit);

        tokio::spawn(async move {
            let (link_snd, mut link_rcv) = unbounded_channel();
            let visited = Arc::new(Mutex::new(HashSet::new()));
            let mut visit_fut = vec![Box::pin(Self::visit(
                site,
                visited.clone(),
                link_snd.clone(),
            ))];
            loop {
                let (page, _, mut rem) = select_all(visit_fut).await;
                if let Some(page) = page {
                    page_snd.send(page).await.unwrap();
                }

                // no links to process
                if rem.is_empty() && link_rcv.is_empty() {
                    break;
                }

                // fits the rate and contains smth to process
                while !link_rcv.is_empty() && rem.len() < rate_limit {
                    let new_link = link_rcv.recv().await.unwrap();
                    rem.push(Box::pin(Self::visit(
                        new_link,
                        visited.clone(),
                        link_snd.clone(),
                    )))
                }
                visit_fut = rem;
            }
        });

        page_rcv
    }

    async fn visit(
        site: String,
        visited: Arc<Mutex<HashSet<String>>>,
        link_snd: UnboundedSender<String>,
    ) -> Option<Page> {
        if !visited.lock().unwrap().insert(site.clone()) {
            return None;
        }
        let content = reqwest::get(site.clone())
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);
        let links = finder.links(&content).map(|l| l.as_str().to_string());
        for link in links {
            link_snd.send(link.clone()).unwrap();
        }
        Some(Page {
            url: site,
            body: content,
        })
    }
}
