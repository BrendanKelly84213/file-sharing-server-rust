#[allow(dead_code)]
pub struct FEntry {
    filename: String,
    filesize: i16,
    first_block: i16, // Pointers to data blocks
}

#[allow(dead_code)]
impl FEntry {
    pub fn new(filename: String, filesize: i16, first_block: i16) -> Result<Self, String> {
        // Check filename is max 11 bytes long
        if filename.chars().count() > 11 {
            return Err("Filename cannot be longer than 11 characters.".to_string());
        }
        Ok(Self {
            filename,
            filesize,
            first_block,
        })
    }

    // Getters and Setters
    pub fn get_filename(&self) -> &str {
        &self.filename
    }

    pub fn set_filename(&mut self, filename: String) -> Result<(), String> {
        if filename.chars().count() > 11 {
            return Err("Filename cannot be longer than 11 characters.".to_string());
        }
        self.filename = filename;
        Ok(())
    }

    pub fn get_filesize(&self) -> i16 {
        self.filesize
    }

    pub fn set_filesize(&mut self, filesize: i16) -> Result<(), String> {
        if filesize < 0 {
            return Err("Filesize cannot be negative.".to_string());
        }
        self.filesize = filesize;
        Ok(())
    }

    pub fn get_first_block(&self) -> i16 {
        self.first_block
    }
}
