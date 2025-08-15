use openssl::symm::{Cipher, Crypter, Mode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EncryptionType {
    Aes256Cbc,
    Aes256Ctr,
    Aes128Ctr,
    Aes128Cbc
}

pub struct DataCipher {
    encryption_type: EncryptionType,
    key: Vec<u8>,
}

impl DataCipher {

    pub fn new_init(encryption_type: EncryptionType, key: String) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let hash = hasher.finalize().to_vec();
        Self { encryption_type, key: hash }
    }

    /// Get the OpenSSL cipher enum
    fn get_cipher(&self) -> Cipher {
        match self.encryption_type {
            EncryptionType::Aes256Cbc => Cipher::aes_256_cbc(),
            EncryptionType::Aes256Ctr => Cipher::aes_256_ctr(),
            EncryptionType::Aes128Cbc => Cipher::aes_128_cbc(),
            EncryptionType::Aes128Ctr => Cipher::aes_128_ctr(),
        }
    }

    /// Calculate required output buffer size
    pub fn required_buffer_size(&self, input_len: usize) -> usize {
        match self.encryption_type {
            EncryptionType::Aes256Ctr | EncryptionType::Aes128Ctr => input_len, // Stream cipher: no padding
            EncryptionType::Aes256Cbc | EncryptionType::Aes128Cbc => {
                let block_size = self.get_cipher().block_size();
                let rem = input_len % block_size;
                if rem == 0 {
                    input_len
                } else {
                    input_len + (block_size - rem)
                }
            }
        }
    }

    pub fn encrypt_block(&self, input: &[u8], output: &mut [u8], iv: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        self.crypt(input, output, iv, Mode::Encrypt)
    }

    pub fn decrypt_block(&self, input: &[u8], output: &mut [u8], iv: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        self.crypt(input, output, iv, Mode::Decrypt)
    }

    fn crypt(
        &self,
        input: &[u8],
        output: &mut [u8],
        iv: &[u8],
        mode: Mode,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let cipher = self.get_cipher();
        let key_len = cipher.key_len();
        let iv_len = cipher.iv_len().unwrap_or(16);

        if self.key.len() < key_len {
            return Err("Key is too short for the selected cipher".into());
        }

        if iv.len() < iv_len {
            return Err("IV is too short for the selected cipher".into());
        }

        let mut crypter = Crypter::new(cipher, mode, &self.key[..key_len], Some(&iv[..iv_len]))?;
        /* 
        match self.vpn_config.encryption_type {
            EncryptionType::Aes256Ctr | EncryptionType::Aes128Ctr => crypter.pad(false),
            EncryptionType::Aes256Cbc | EncryptionType::Aes128Cbc => crypter.pad(true)
        };
        */
        crypter.pad(true);

        let mut count = crypter.update(input, output)?;
        count += crypter.finalize(&mut output[count..])?;

        Ok(count)
    }
}
