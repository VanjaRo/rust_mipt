#![forbid(unsafe_code)]
use std::{
    fs,
    io::{self, Error, ErrorKind::Unsupported},
    path::Path,
    usize,
};

use crate::handle::{DirHandle, FileHandle, Handle};

type Callback<'a> = dyn FnMut(&mut Handle) + 'a;

#[derive(Default)]
pub struct Walker<'a> {
    callbacks: Vec<Box<Callback<'a>>>,
}

impl<'a> Walker<'a> {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Handle) + 'a,
    {
        self.callbacks.push(Box::new(callback));
    }

    pub fn walk<P: AsRef<Path>>(mut self, path: P) -> io::Result<()> {
        if self.callbacks.is_empty() {
            return Ok(());
        }
        let cb_idxs = (0..self.callbacks.len()).collect::<Vec<_>>();
        self.walk_with_callbacks(path, &cb_idxs)
    }

    fn walk_with_callbacks<P: AsRef<Path>>(&mut self, path: P, idxs: &[usize]) -> io::Result<()> {
        let mut handler = if path.as_ref().is_dir() {
            Handle::Dir(DirHandle::new(path.as_ref()))
        } else if path.as_ref().is_file() {
            Handle::File(FileHandle::new(path.as_ref()))
        } else {
            return Err(Error::from(Unsupported));
        };

        let new_cb_idxs = self.process_callbacks(&mut handler, idxs);
        if new_cb_idxs.is_empty() {
            return Ok(());
        }

        match handler {
            Handle::Dir(dir_handle) => self.process_handle_dir(dir_handle, &new_cb_idxs),
            Handle::File(file_handle) => self.process_handle_file(file_handle, &new_cb_idxs),
            _ => Ok(()),
        }
    }

    fn process_callbacks(&mut self, mut handler: &mut Handle, idxs: &[usize]) -> Vec<usize> {
        idxs.into_iter()
            .map(|&idx| {
                self.callbacks[idx](&mut handler);
                let processed = Self::handler_processed_and_reset(&mut handler);
                (idx, processed)
            })
            .filter_map(|(idx, do_repeat)| do_repeat.then_some(idx))
            .collect::<Vec<_>>()
    }

    fn handler_processed_and_reset(handler: &mut Handle) -> bool {
        match handler {
            Handle::Dir(dir_handle) => {
                let processed = dir_handle.processed();
                dir_handle.reset();
                processed
            }
            Handle::File(file_handle) => {
                let processed = file_handle.processed();
                file_handle.reset();
                processed
            }
            _ => false,
        }
    }

    fn process_handle_dir(&mut self, dir_handle: DirHandle, idxs: &[usize]) -> io::Result<()> {
        dir_handle.path().read_dir().and_then(|mut read_dir| {
            read_dir.try_for_each(|entry_opt| {
                entry_opt.and_then(|entry| self.walk_with_callbacks(entry.path(), idxs))
            })
        })
    }

    fn process_handle_file(&mut self, file_handle: FileHandle, idxs: &[usize]) -> io::Result<()> {
        fs::read(file_handle.path()).and_then(|content| {
            let mut content_handle = Handle::Content {
                file_path: file_handle.path(),
                content: &content,
            };
            self.process_callbacks(&mut content_handle, idxs);
            Ok(())
        })
    }
}
