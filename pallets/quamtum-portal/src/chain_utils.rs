use parity_scale_codec::Encode;
use crate::chain_utils::ChainRequestError::ConversionError;

pub struct ChainUtils;
use sp_std::{str};
use sp_std::prelude::*;
use ethabi_nostd::{U256, H256}; //vec::{Vec};
use numtoa::NumToA;

#[derive(Debug)]
pub enum ChainRequestError {
    ErrorGettingJsonRpcResponse,
    BadRemoteData,
    ConversionError,
	ErrorCreatingTransaction,
	RemoteBlockAlreadyMined,
}

pub trait ToJson {
    type BaseType;
    fn to_json(&self) -> Vec<u8>;
}

pub type ChainRequestResult<T> = Result<T, ChainRequestError>;

fn u64_to_str(num: u64) -> Vec<u8> {
    let mut num_buffer = [0u8; 20];
    num.numtoa_str(10, &mut num_buffer);
    log::info!("num2str1 : {:?}", num_buffer);
    let mut s: Vec<u8> = Vec::new();
    num_buffer.into_iter().filter(|u| *u != 0)
        .for_each(|u| { s.push(u); () });
    log::info!("num2str2 : {:?}", s);
    s
}

const HEX_TABLE: [u8;16] = [
    '0' as u8, '1' as u8, '2' as u8, '3' as u8, '4' as u8, '5' as u8,
    '6' as u8, '7' as u8, '8' as u8, '9' as u8, 'a' as u8, 'b' as u8,
    'c' as u8, 'd' as u8, 'e' as u8, 'f' as u8];

impl ChainUtils {
    pub fn hex_to_u64(s: &[u8]) -> Result<u64, ChainRequestError> {
        if s.len() < 2 {
            return Err(ChainRequestError::ConversionError);
        }
        let hexb = if s[0] == '0' as u8 && s[1] == 'x' as u8 {
            &s[2..] } else { s };
        let hex = str::from_utf8(&hexb)
            .map_err(|e| {
                log::error!("Error when converting from hex: {:?}", e);
                ChainRequestError::ConversionError
            })?;
        let rv = u64::from_str_radix(hex, 16)
            .map_err(|e| {
                log::error!("{:?}", e);
                ChainRequestError::ConversionError
            })?;
        Ok(rv)
    }

    pub fn bytes_to_hex(s: &[u8]) -> Vec<u8> {
        let mut rv = Vec::new();
        s.into_iter().for_each(|u| {
            rv.push(HEX_TABLE[((u & 0xf0) >> 4) as usize]);
            rv.push(HEX_TABLE[(u & 0x0f) as usize]);
        });
        rv
    }

    pub fn hex_add_0x(s: &[u8]) -> Vec<u8> {
        if s.len() >= 2 && s[0] == '0' as u8 && s[1] == 'x' as u8 {
            return Vec::from(s.clone());
        }
        let mut zx = vec!['0' as u8, 'x' as u8];
        zx.extend(s);
        zx
    }

    pub fn u256_to_hex_0x(i: &U256) -> Vec<u8> {
        let fmted = i.encode();
        let mut zx = vec!['0' as u8, 'x' as u8];
        zx.extend(fmted.into_iter().map(|i| i + '0' as u8));
        // println!("FMTED {:?}", &zx);
        zx
    }

    pub fn u256_to_hex_str(i: &U256) -> Vec<u8> {
        let fmted = i.encode();
        let mut zx = vec![];
        zx.extend(fmted.into_iter().map(|i| i + '0' as u8));
        zx
    }

    pub fn h256_to_hex_0x(i: &H256) -> Vec<u8> {
        let fmted = i.encode();
        let mut zx = vec!['0' as u8, 'x' as u8];
        zx.extend(fmted.into_iter().map(|i| i + '0' as u8));
        zx
    }
}

pub struct JsonSer {
    buff: Vec<u8>,
    empty: bool,
}

impl JsonSer {
    pub fn new() -> Self {
        JsonSer { buff: Vec::new(), empty: true }
    }

    pub fn start(&mut self) -> &mut Self {
        if self.buff.len() > 0 { panic!("JSON already started") }
        self.buff.push('{' as u8);
        self
    }

    pub fn end(&mut self) -> &mut Self {
        self.buff.push('}' as u8);
        self
    }

    pub fn comma(&mut self) -> &mut Self {
        if !self.empty {
            self.buff.push(',' as u8);
        }
        self
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.comma();
        self.buff.push('"' as u8);
        name.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.buff.push('"' as u8);
        self.buff.push(':' as u8);
        self
    }

    pub fn arr_string(&mut self, val: &str) -> &mut Self {
        self.comma();
        self.buff.push('"' as u8);
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.buff.push('"' as u8);
        self.empty = false;
        self
    }

    pub fn arr_val(&mut self, val: &str) -> &mut Self {
        self.comma();
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.empty = false;
        self
    }

    pub fn string(&mut self, name: &str, val: &str) -> &mut Self {
        self.name(name);
        self.buff.push('"' as u8);
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.buff.push('"' as u8);
        self.empty = false;
        self
    }

    pub fn num(&mut self, name: &str, val: u64) -> &mut Self {
        let v = u64_to_str(val);
        self.val(name, str::from_utf8(&v.as_slice()).unwrap())
    }

    pub fn val(&mut self, name: &str, val: &str) -> &mut Self {
        self.name(name);
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.empty = false;
        self
    }

    pub fn arr(&mut self, name: &str, val: &str) -> &mut Self {
        self.name(name);
        self.arr_start();
        val.as_bytes().into_iter().for_each(|b| self. buff.push(b.clone()));
        self.arr_end();
        self.empty = false;
        self
    }

    pub fn arr_start(&mut self) -> &mut Self {
        self.buff.push('[' as u8);
        self.empty = true;
        self
    }

    pub fn arr_end(&mut self) -> &mut Self {
        self.buff.push(']' as u8);
        self.empty = false;
        self
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.buff.clone()
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use crate::chain_utils::JsonSer;
    use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};

    #[test]
    fn jsonify_num() {
        let jo = JsonSer::new()
            .start()
            .num("id", 1)
            .end()
            .to_vec();
        log::info!("Jos is {:?}", jo);
        println!("Jo is {:?}", jo);
        let jos = str::from_utf8(jo.as_slice());
        println!("Jos is {}", jos.unwrap());
    }
}