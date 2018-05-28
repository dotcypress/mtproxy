extern crate rand;

use std::io;

use bytes::{Buf, IntoBuf};
use crypto::aes::{self, KeySize};
use crypto::symmetriccipher::SynchronousStreamCipher;
use crypto::{digest::Digest, sha2::Sha256};
use rand::RngCore;

pub struct Proto {
  seed: Vec<u8>,
  dc_idx: usize,
  dec: Box<SynchronousStreamCipher>,
  enc: Box<SynchronousStreamCipher>,
}

impl Proto {
  pub fn new() -> Proto {
    let mut buf = vec![0u8; 64];
    let mut rng = rand::thread_rng();
    loop {
      rng.fill_bytes(&mut buf);
      let check = ((buf[7] as u32) << 24)
        | ((buf[6] as u32) << 16)
        | ((buf[5] as u32) << 8)
        | (buf[4] as u32);
      if buf[0] == 0xef || check == 0 {
        continue;
      }
      let check = ((buf[3] as u32) << 24)
        | ((buf[2] as u32) << 16)
        | ((buf[1] as u32) << 8)
        | (buf[0] as u32);
      match check {
        0x44414548 | 0x54534f50 | 0x20544547 | 0x4954504f | 0xeeeeeeee => (),
        _ => break,
      }
    }

    buf[56] = 0xef;
    buf[57] = 0xef;
    buf[58] = 0xef;
    buf[59] = 0xef;

    let key_iv_enc = buf[8..56].to_vec();
    let key_iv_dec: Vec<u8> = key_iv_enc.iter().rev().cloned().collect();
    let mut enc = aes::ctr(KeySize::KeySize256, &key_iv_enc[0..32], &key_iv_enc[32..48]);
    let dec = aes::ctr(KeySize::KeySize256, &key_iv_dec[0..32], &key_iv_dec[32..48]);

    let mut buf_enc = vec![0u8; 64];
    enc.process(&buf, &mut buf_enc);
    for n in 56..64 {
      buf[n] = buf_enc[n];
    }

    Proto {
      seed: buf,
      dc_idx: 0,
      dec,
      enc,
    }
  }

  pub fn from_seed(buf: &[u8], secret: &[u8]) -> io::Result<Proto> {
    let mut hash = Sha256::new();
    let mut dec_key = vec![0u8; hash.output_bytes()];
    hash.input(&[&buf[8..40], &secret].concat());
    hash.result(&mut dec_key);
    let key_iv_rev: Vec<u8> = buf[8..56].iter().cloned().rev().collect();

    let mut hash = Sha256::new();
    let mut enc_key = vec![0u8; hash.output_bytes()];
    hash.input(&[&key_iv_rev[0..32], &secret].concat());
    hash.result(&mut enc_key);

    let mut dec = aes::ctr(KeySize::KeySize256, &dec_key, &buf[40..56]);
    let enc = aes::ctr(KeySize::KeySize256, &enc_key, &key_iv_rev[32..48]);
    let mut buf_dec = vec![0u8; buf.len()];
    dec.process(&buf, &mut buf_dec);
    if buf_dec[56] != 0xef || buf_dec[57] != 0xef || buf_dec[58] != 0xef || buf_dec[59] != 0xef {
      return Err(io::Error::new(io::ErrorKind::Other, "Unknown protocol"));
    }
    let mut dc = buf_dec[60..62].into_buf().get_i16_le().abs();
    if dc == 0  {
      warn!("Unsupported DC index: #0. using #1");
      dc = 1;
    }
    if dc > 5 {
      return Err(io::Error::new(io::ErrorKind::Other, format!("Unsupported DC index: {}", dc)));
    }
    let dc_idx = (dc - 1) as usize;
    Ok(Proto {
      seed: buf.to_vec(),
      dc_idx,
      dec,
      enc,
    })
  }

  pub fn seed(&self) -> &[u8] {
    &self.seed
  }

  pub fn dc(&self) -> usize {
    self.dc_idx
  }

  pub fn dec(&mut self, input: &[u8], output: &mut [u8]) {
    self.dec.process(input, output);
  }

  pub fn enc(&mut self, input: &[u8], output: &mut [u8]) {
    self.enc.process(input, output);
  }
}
