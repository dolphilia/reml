use std::fs::OpenOptions;

/// `File::create` などで使用するオープンオプション。
#[derive(Debug, Clone)]
pub struct FileOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

impl FileOptions {
    pub fn new() -> Self {
        Self {
            read: true,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    pub fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    pub fn read_enabled(&self) -> bool {
        self.read
    }

    pub fn write_enabled(&self) -> bool {
        self.write || self.append || self.truncate || self.create || self.create_new
    }

    pub(crate) fn apply_to(&self, opts: &mut OpenOptions) {
        opts.read(self.read)
            .write(self.write)
            .append(self.append)
            .truncate(self.truncate)
            .create(self.create)
            .create_new(self.create_new);
    }

    pub(crate) fn snapshot(&self) -> FileOptionsSnapshot {
        FileOptionsSnapshot {
            read: self.read,
            write: self.write,
            append: self.append,
            truncate: self.truncate,
            create: self.create,
            create_new: self.create_new,
        }
    }
}

impl Default for FileOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// 実際に利用したオプションを保持するスナップショット。
#[derive(Debug, Clone)]
pub struct FileOptionsSnapshot {
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) append: bool,
    pub(crate) truncate: bool,
    pub(crate) create: bool,
    pub(crate) create_new: bool,
}

impl FileOptionsSnapshot {
    pub fn read(&self) -> bool {
        self.read
    }

    pub fn write(&self) -> bool {
        self.write
    }

    pub fn append(&self) -> bool {
        self.append
    }

    pub fn truncate(&self) -> bool {
        self.truncate
    }

    pub fn create(&self) -> bool {
        self.create
    }

    pub fn create_new(&self) -> bool {
        self.create_new
    }
}
