use std::string::ToString;

use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use byteorder::{BigEndian, ByteOrder};
use sha1::{Digest, Sha1};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum CryptoError {
    #[error("invalid aes key, reason: {0}")]
    InvalidAesKey(&'static str),
    #[error("invalid decrypt data, reason: {0}")]
    InvalidDecryptData(&'static str),
}

#[derive(Debug)]
pub(crate) struct Crypto {
    token: String,
    aes_key: Vec<u8>,
}

pub(crate) struct Payload {
    pub data: Vec<u8>,
    pub receiver_id: Vec<u8>,
}

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

impl Crypto {
    pub(crate) fn new(
        token: impl ToString,
        encoding_aes_key: impl AsRef<[u8]>,
    ) -> Result<Crypto, CryptoError> {
        let bytes = encoding_aes_key.as_ref();
        if bytes.len() != 43 {
            return Err(CryptoError::InvalidAesKey("length must be 43"));
        }
        let mut buf = Vec::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        buf.push(b'=');
        let aes_key = base64::decode(&buf)
            .map_err(|_| CryptoError::InvalidAesKey("invalid base64 string"))?;
        let token = token.to_string();
        Ok(Crypto { token, aes_key })
    }

    pub(crate) fn sign(&self, msg_encrypt: String, timestamp: u64, nonce: u64) -> String {
        let time_s = format!("{}", timestamp);
        let nonce_s = format!("{}", nonce);

        let mut items = vec![self.token.clone(), time_s, nonce_s, msg_encrypt];
        items.sort();
        let data = items.join("");

        let mut hasher = Sha1::new();
        hasher.input(data.as_bytes());
        hex::encode(hasher.result())
    }

    pub(crate) fn encrypt(&self, payload: &Payload) -> String {
        let aes_key = &self.aes_key;
        let iv = &aes_key[0..16];

        let data_len = payload.data.len();
        let recv_id_len = payload.receiver_id.len();
        let mut buf = Vec::with_capacity(20 + data_len + recv_id_len);
        buf.extend_from_slice(&[0; 20]);
        BigEndian::write_u32(&mut buf[16..], data_len as u32);
        buf.extend_from_slice(&payload.data);
        buf.extend_from_slice(&payload.receiver_id);

        let cipher = Aes256Cbc::new_var(&aes_key, &iv).unwrap();
        let encrypted = cipher.encrypt_vec(&buf);
        base64::encode(encrypted)
    }

    pub(crate) fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<Payload, CryptoError> {
        // TODO: get this from cipher
        let block_size = 16;

        let aes_msg = base64::decode(data)
            .map_err(|_| CryptoError::InvalidDecryptData("invalid base64 string"))?;

        let aes_key = &self.aes_key;
        let iv = &aes_key[0..block_size];

        let cipher = Aes256Cbc::new_var(&aes_key, &iv).unwrap();
        let decrypted = cipher
            .decrypt_vec(&aes_msg)
            .map_err(|_| CryptoError::InvalidDecryptData("invalid length"))?;
        let msg_len = BigEndian::read_u32(&decrypted[16..20]) as usize;
        let rcv_id_idx = 20 + msg_len;
        let data = Vec::from(&decrypted[20..rcv_id_idx]);
        let receiver_id = Vec::from(&decrypted[rcv_id_idx..]);
        Ok(Payload { data, receiver_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_aes_key() {
        let token = "QDG6eK";
        let k1 = "123";
        let r1 = Crypto::new(token, k1);
        assert!(r1.is_err());

        // invalid base64: invalid last byte
        let k2 = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIwC";
        let r2 = Crypto::new(token, k2);
        assert!(r2.is_err());
    }

    #[test]
    fn test_invalid_decrypt_data() {
        let token = "QDG6eK";
        let k1 = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIws";

        let crypto = Crypto::new(token, k1).unwrap();
        let r1 = crypto.decrypt("123");
        assert!(r1.is_err());

        let r2 = crypto.decrypt("0123456789abcdef");
        assert!(r2.is_err());
    }

    #[test]
    fn test_sign() {
        let token = "QDG6eK";
        let msg_encrypt = "RypEvHKD8QQKFhvQ6QleEB4J58tiPdvo+rtK1I9qca6aM/wvqnLSV5zEPeusUiX5L5X/0lWfrf0QADHHhGd3QczcdCUpj911L3vg3W/sYYvuJTs3TUUkSUXxaccAS0qhxchrRYt66wiSpGLYL42aM6A8dTT+6k4aSknmPj48kzJs8qLjvd4Xgpue06DOdnLxAUHzM6+kDZ+HMZfJYuR+LtwGc2hgf5gsijff0ekUNXZiqATP7PF5mZxZ3Izoun1s4zG4LUMnvw2r+KqCKIw+3IQH03v+BCA9nMELNqbSf6tiWSrXJB3LAVGUcallcrw8V2t9EL4EhzJWrQUax5wLVMNS0+rUPA3k22Ncx4XXZS9o0MBH27Bo6BpNelZpS+/uh9KsNlY6bHCmJU9p8g7m3fVKn28H3KDYA5Pl/T8Z1ptDAVe0lXdQ2YoyyH2uyPIGHBZZIs2pDBS8R07+qN+E7Q==";
        let encoding_aes_key = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIws";
        let timestamps = 1409659813;
        let nonce = 1372623149;

        let crypto = Crypto::new(token, encoding_aes_key).unwrap();

        let sign = crypto.sign(msg_encrypt.to_string(), timestamps, nonce);
        assert_eq!(sign, "477715d11cdb4164915debcba66cb864d751f3e6");
    }

    #[test]
    fn test_decrypt() {
        let token = "QDG6eK";
        let msg_encrypt: &str =
            "6KmUQuPVu7UhjyVqRdbo5SfcRqaHvbUlKSHFvBV2ZuR6TIlKsygcfeSd1GDplg1C5KSKr6UPHCaC/nIX3ZNt9w==";
        let encoding_aes_key = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIws";
        let msg_data: &str = "94966531020182955848408";

        let crypto = Crypto::new(token, encoding_aes_key).unwrap();
        let msg = crypto.decrypt(msg_encrypt).unwrap();
        let data = String::from_utf8(msg.data).unwrap();
        assert_eq!(data, msg_data);
    }

    #[test]
    fn test_crypto() {
        let token = "QDG6eK";
        let encoding_aes_key = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIws";
        let data = "foobarbaz123456788";
        let receiver = "123";
        let payload = Payload {
            data: Vec::from(data),
            receiver_id: Vec::from(receiver),
        };

        let crypto = Crypto::new(token, encoding_aes_key).unwrap();
        let encrypted = crypto.encrypt(&payload);
        let ret = crypto.decrypt(encrypted).unwrap();

        assert_eq!(ret.data, payload.data);
        assert_eq!(ret.receiver_id, payload.receiver_id);
    }
}
