use crate::chain_utils::ChainRequestError::ConversionError;

pub struct ChainUtils;
use sp_std::{str};

#[derive(Debug)]
pub enum ChainRequestError {
    ErrorGettingJsonRpcResponse,
    BadRemoteData,
    ConversionError,
	ErrorCreatingTransaction,
}

pub type ChainRequestResult<T> = Result<T, ChainRequestError>;

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
}
