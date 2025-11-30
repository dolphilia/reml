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
}

impl Default for FileOptions {
    fn default() -> Self {
        Self::new()
    }
}
