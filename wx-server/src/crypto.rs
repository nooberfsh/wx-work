use byteorder::{BigEndian, ByteOrder};
use itertools::Itertools;
use openssl::symm::Cipher;
use sha1::{Digest, Sha1};

use crate::{Error, Result};

#[derive(Debug)]
pub(crate) struct Crypto {
    token: String,
    aes_key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Payload {
    pub data: Vec<u8>,
    pub receive_id: Vec<u8>,
}

impl Crypto {
    pub(crate) fn new(token: String, encoding_aes_key: String) -> Result<Crypto> {
        let bytes = encoding_aes_key.as_bytes();
        if bytes.len() != 43 {
            return Err(Error::InvalidAesKey);
        }
        let mut buf = Vec::with_capacity(bytes.len());
        buf.extend_from_slice(bytes);
        buf.push(b'=');
        let aes_key = base64::decode(&buf)?;
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
        hasher
            .result()
            .iter()
            .format_with("", |byte, f| f(&format_args!("{:02x}", byte)))
            .to_string()
    }

    pub(crate) fn encrypt(&self, msg: Payload) -> Result<String> {
        let aes_key = &self.aes_key;
        let iv = &aes_key[0..16];

        let data_len = msg.data.len();
        let recv_id_len = msg.receive_id.len();
        let mut buf = Vec::with_capacity(20 + data_len + recv_id_len);
        buf.extend_from_slice(&[0; 20]);
        BigEndian::write_u32(&mut buf[16..], data_len as u32);
        buf.extend_from_slice(&msg.data);
        buf.extend_from_slice(&msg.receive_id);

        let cipher = Cipher::aes_256_cbc();
        let encrypted = openssl::symm::encrypt(cipher, &aes_key, Some(iv), &buf)?;
        Ok(base64::encode(encrypted))
    }

    pub fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<Payload> {
        let aes_key = &self.aes_key;
        let iv = &aes_key[0..16];

        let aes_msg = base64::decode(data)?;
        let cipher = Cipher::aes_256_cbc();
        let decrypted = openssl::symm::decrypt(cipher, &aes_key, Some(iv), &aes_msg)?;
        let msg_len = BigEndian::read_u32(&decrypted[16..20]) as usize;
        let rcv_id_idx = 20 + msg_len;

        Ok(Payload {
            data: Vec::from(&decrypted[20..rcv_id_idx]),
            receive_id: Vec::from(&decrypted[rcv_id_idx..]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_aes_key() {
        let token = "QDG6eK";
        let k1 = "123";
        let r1 = Crypto::new(token.to_string(), k1.to_string());
        assert!(r1.is_err());

        // invalid base64: invalid last byte
        let k2 = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIwC";
        let r2 = Crypto::new(token.to_string(), k2.to_string());
        assert!(r2.is_err());
    }

    #[test]
    fn test_sign() {
        let token = "QDG6eK";
        let msg_encrypt = "RypEvHKD8QQKFhvQ6QleEB4J58tiPdvo+rtK1I9qca6aM/wvqnLSV5zEPeusUiX5L5X/0lWfrf0QADHHhGd3QczcdCUpj911L3vg3W/sYYvuJTs3TUUkSUXxaccAS0qhxchrRYt66wiSpGLYL42aM6A8dTT+6k4aSknmPj48kzJs8qLjvd4Xgpue06DOdnLxAUHzM6+kDZ+HMZfJYuR+LtwGc2hgf5gsijff0ekUNXZiqATP7PF5mZxZ3Izoun1s4zG4LUMnvw2r+KqCKIw+3IQH03v+BCA9nMELNqbSf6tiWSrXJB3LAVGUcallcrw8V2t9EL4EhzJWrQUax5wLVMNS0+rUPA3k22Ncx4XXZS9o0MBH27Bo6BpNelZpS+/uh9KsNlY6bHCmJU9p8g7m3fVKn28H3KDYA5Pl/T8Z1ptDAVe0lXdQ2YoyyH2uyPIGHBZZIs2pDBS8R07+qN+E7Q==";
        let encoding_aes_key = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIws";
        let timestamps = 1409659813;
        let nonce = 1372623149;

        let crypto = Crypto::new(token.to_string(), encoding_aes_key.to_string()).unwrap();

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
        let msg_recv_id: &str = "ww6a112864f8022910";

        let crypto = Crypto::new(token.to_string(), encoding_aes_key.to_string()).unwrap();

        let msg = crypto.decrypt(msg_encrypt).unwrap();
        let data = String::from_utf8(msg.data).unwrap();
        let recv_id = String::from_utf8(msg.receive_id).unwrap();
        assert_eq!(data, msg_data);
        assert_eq!(recv_id, msg_recv_id);
    }

    #[test]
    fn test_crypto() {
        let token = "QDG6eK";
        let encoding_aes_key = "4Ma3YBrSBbX2aez8MJpXGBne5LSDwgGqHbhM9WPYIws";
        let data = "foobarbaz123456788";
        let recv_id = "ww6a112864f8022910";

        let crypto = Crypto::new(token.to_string(), encoding_aes_key.to_string()).unwrap();

        let msg = Payload {
            data: Vec::from(data),
            receive_id: Vec::from(recv_id),
        };
        let encrypted = crypto.encrypt(msg).unwrap();

        let msg = crypto.decrypt(encrypted).unwrap();
        let ret_data = String::from_utf8(msg.data).unwrap();
        let ret_recv_id = String::from_utf8(msg.receive_id).unwrap();

        assert_eq!(ret_data, data);
        assert_eq!(ret_recv_id, recv_id);
    }
}
