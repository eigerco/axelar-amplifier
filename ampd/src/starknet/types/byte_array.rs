pub struct ByteArray {
    data: Vec<u8>,
    pending_word: [u8; 30],
    pending_word_lent: usize,
}

impl ByteArray {}
