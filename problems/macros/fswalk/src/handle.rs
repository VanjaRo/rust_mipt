#![forbid(unsafe_code)]

use std::path::Path;

pub enum Handle<'a> {
    Dir(DirHandle<'a>),
    File(FileHandle<'a>),
    Content {
        file_path: &'a Path,
        content: &'a [u8],
    },
}

pub struct DirHandle<'a> {
    path: &'a Path,
    processed: bool,
}

impl<'a> DirHandle<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self {
            path,
            processed: false,
        }
    }

    pub fn processed(&self) -> bool {
        self.processed
    }
    pub fn reset(&mut self) {
        self.processed = false;
    }

    pub fn descend(&mut self) {
        self.processed = true
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct FileHandle<'a> {
    path: &'a Path,
    processed: bool,
}

impl<'a> FileHandle<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self {
            path,
            processed: false,
        }
    }

    pub fn read(&mut self) {
        self.processed = true
    }

    pub fn processed(&self) -> bool {
        self.processed
    }

    pub fn reset(&mut self) {
        self.processed = false;
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
