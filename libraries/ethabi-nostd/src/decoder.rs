// Copyright 2015-2020 Parity Technologies
// Copyright 2020 Snowfork
//
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 <LICENSE or
// http://www.apache.org/licenses/LICENSE-2.0>. This file may not be
// copied, modified, or distributed except according to those terms.

//! ABI decoder.

use crate::{util::slice_data, Error, ParamKind, Token, Word};

use sp_std::prelude::*; //vec::{Vec};

struct DecodeResult {
	token: Token,
	new_offset: usize,
}

struct BytesTaken {
	bytes: Vec<u8>,
	new_offset: usize,
}

fn as_u32(slice: &Word) -> Result<u32, Error> {
	if !slice[..28].iter().all(|x| *x == 0) {
		return Err(Error::InvalidData);
	}

	let result = ((slice[28] as u32) << 24)
		+ ((slice[29] as u32) << 16)
		+ ((slice[30] as u32) << 8)
		+ (slice[31] as u32);

	Ok(result)
}

fn as_bool(slice: &Word) -> Result<bool, Error> {
	if !slice[..31].iter().all(|x| *x == 0) {
		return Err(Error::InvalidData);
	}

	Ok(slice[31] == 1)
}

/// Decodes ABI compliant vector of bytes into vector of tokens described by types param.
pub fn decode(types: &[ParamKind], data: &[u8]) -> Result<Vec<Token>, Error> {
	let is_empty_bytes_valid_encoding = types.iter().all(|t| t.is_empty_bytes_valid_encoding());
	if !is_empty_bytes_valid_encoding && data.is_empty() {
		return Err(Error::InvalidName);
	}
	let slices = slice_data(data)?;
	let mut tokens = Vec::with_capacity(types.len());
	let mut offset = 0;
	for param in types {
		let res = decode_param(param, &slices, offset)?;
		offset = res.new_offset;
		tokens.push(res.token);
	}
	Ok(tokens)
}

fn peek(slices: &[Word], position: usize) -> Result<&Word, Error> {
	slices.get(position).ok_or(Error::InvalidData)
}

fn take_bytes(slices: &[Word], position: usize, len: usize) -> Result<BytesTaken, Error> {
	let slices_len = (len + 31) / 32;

	let mut bytes_slices = Vec::with_capacity(slices_len);
	for i in 0..slices_len {
		let slice = peek(slices, position + i)?;
		bytes_slices.push(slice);
	}

	let bytes = bytes_slices.into_iter().flat_map(|slice| slice.to_vec()).take(len).collect();

	let taken = BytesTaken { bytes, new_offset: position + slices_len };

	Ok(taken)
}

fn decode_param(param: &ParamKind, slices: &[Word], offset: usize) -> Result<DecodeResult, Error> {
	match *param {
		ParamKind::Address => {
			let slice = peek(slices, offset)?;
			let mut address = [0u8; 20];
			address.copy_from_slice(&slice[12..]);

			let result =
				DecodeResult { token: Token::Address(address.into()), new_offset: offset + 1 };

			Ok(result)
		},
		ParamKind::Int(_) => {
			let slice = peek(slices, offset)?;

			let result =
				DecodeResult { token: Token::Int((*slice).into()), new_offset: offset + 1 };

			Ok(result)
		},
		ParamKind::Uint(_) => {
			let slice = peek(slices, offset)?;

			let result =
				DecodeResult { token: Token::Uint((*slice).into()), new_offset: offset + 1 };

			Ok(result)
		},
		ParamKind::Bool => {
			let slice = peek(slices, offset)?;

			let b = as_bool(slice)?;

			let result = DecodeResult { token: Token::Bool(b), new_offset: offset + 1 };
			Ok(result)
		},
		ParamKind::FixedBytes(len) => {
			// FixedBytes is anything from bytes1 to bytes32. These values
			// are padded with trailing zeros to fill 32 bytes.
			let taken = take_bytes(slices, offset, len)?;
			let result = DecodeResult {
				token: Token::FixedBytes(taken.bytes),
				new_offset: taken.new_offset,
			};
			Ok(result)
		},
		ParamKind::Bytes => {
			let offset_slice = peek(slices, offset)?;
			let len_offset = (as_u32(offset_slice)? / 32) as usize;

			let len_slice = peek(slices, len_offset)?;
			let len = as_u32(len_slice)? as usize;

			let taken = take_bytes(slices, len_offset + 1, len)?;

			let result = DecodeResult { token: Token::Bytes(taken.bytes), new_offset: offset + 1 };
			Ok(result)
		},
		ParamKind::String => {
			let offset_slice = peek(slices, offset)?;
			let len_offset = (as_u32(offset_slice)? / 32) as usize;

			let len_slice = peek(slices, len_offset)?;
			let len = as_u32(len_slice)? as usize;

			let taken = take_bytes(slices, len_offset + 1, len)?;

			let result = DecodeResult { token: Token::String(taken.bytes), new_offset: offset + 1 };
			Ok(result)
		},
		ParamKind::Array(ref t) => {
			let offset_slice = peek(slices, offset)?;
			let len_offset = (as_u32(offset_slice)? / 32) as usize;
			let len_slice = peek(slices, len_offset)?;
			let len = as_u32(len_slice)? as usize;

			let tail = &slices[len_offset + 1..];
			let mut tokens = Vec::with_capacity(len);
			let mut new_offset = 0;

			for _ in 0..len {
				let res = decode_param(t, tail, new_offset)?;
				new_offset = res.new_offset;
				tokens.push(res.token);
			}

			let result = DecodeResult { token: Token::Array(tokens), new_offset: offset + 1 };

			Ok(result)
		},
		ParamKind::FixedArray(ref t, len) => {
			let mut tokens = Vec::with_capacity(len);
			let is_dynamic = param.is_dynamic();

			let (tail, mut new_offset) = if is_dynamic {
				(&slices[(as_u32(peek(slices, offset)?)? as usize / 32)..], 0)
			} else {
				(slices, offset)
			};

			for _ in 0..len {
				let res = decode_param(t, tail, new_offset)?;
				new_offset = res.new_offset;
				tokens.push(res.token);
			}

			let result = DecodeResult {
				token: Token::FixedArray(tokens),
				new_offset: if is_dynamic { offset + 1 } else { new_offset },
			};

			Ok(result)
		},
		ParamKind::Tuple(ref t) => {
			let is_dynamic = param.is_dynamic();

			// The first element in a dynamic Tuple is an offset to the Tuple's data
			// For a static Tuple the data begins right away
			let (tail, mut new_offset) = if is_dynamic {
				(&slices[(as_u32(peek(slices, offset)?)? as usize / 32)..], 0)
			} else {
				(slices, offset)
			};

			let len = t.len();
			let mut tokens = Vec::with_capacity(len);
			for i in 0..len {
				let res = decode_param(&t[i], tail, new_offset)?;
				new_offset = res.new_offset;
				tokens.push(res.token);
			}

			// The returned new_offset depends on whether the Tuple is dynamic
			// dynamic Tuple -> follows the prefixed Tuple data offset element
			// static Tuple  -> follows the last data element
			let result = DecodeResult {
				token: Token::Tuple(tokens),
				new_offset: if is_dynamic { offset + 1 } else { new_offset },
			};

			Ok(result)
		},
	}
}

#[cfg(test)]
mod tests {

	use crate::{decoder::decode, ParamKind, Token};
	use hex_literal::hex;

	#[test]
	fn decode_from_empty_byte_slice() {
		// these can NOT be decoded from empty byte slice
		assert!(decode(&[ParamKind::Address], &[]).is_err());
		assert!(decode(&[ParamKind::Bytes], &[]).is_err());
		assert!(decode(&[ParamKind::Int(0)], &[]).is_err());
		assert!(decode(&[ParamKind::Int(1)], &[]).is_err());
		assert!(decode(&[ParamKind::Int(0)], &[]).is_err());
		assert!(decode(&[ParamKind::Int(1)], &[]).is_err());
		assert!(decode(&[ParamKind::Bool], &[]).is_err());
		assert!(decode(&[ParamKind::String], &[]).is_err());
		assert!(decode(&[ParamKind::Array(Box::new(ParamKind::Bool))], &[]).is_err());
		assert!(decode(&[ParamKind::FixedBytes(1)], &[]).is_err());
		assert!(decode(&[ParamKind::FixedArray(Box::new(ParamKind::Bool), 1)], &[]).is_err());

		// these are the only ones that can be decoded from empty byte slice
		assert!(decode(&[ParamKind::FixedBytes(0)], &[]).is_ok());
		assert!(decode(&[ParamKind::FixedArray(Box::new(ParamKind::Bool), 0)], &[]).is_ok());
	}

	#[test]
	fn decode_static_tuple_of_addresses_and_uints() {
		let encoded = hex!(
			"
			0000000000000000000000001111111111111111111111111111111111111111
			0000000000000000000000002222222222222222222222222222222222222222
			1111111111111111111111111111111111111111111111111111111111111111
		"
		);
		let address1 = Token::Address([0x11u8; 20].into());
		let address2 = Token::Address([0x22u8; 20].into());
		let uint = Token::Uint([0x11u8; 32].into());
		let tuple = Token::Tuple(vec![address1, address2, uint]);
		let expected = vec![tuple];
		let decoded = decode(
			&[ParamKind::Tuple(vec![
				Box::new(ParamKind::Address),
				Box::new(ParamKind::Address),
				Box::new(ParamKind::Uint(32)),
			])],
			&encoded,
		)
		.unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_dynamic_tuple() {
		let encoded = hex!(
			"
			0000000000000000000000000000000000000000000000000000000000000020
			0000000000000000000000000000000000000000000000000000000000000040
			0000000000000000000000000000000000000000000000000000000000000080
			0000000000000000000000000000000000000000000000000000000000000009
			6761766f66796f726b0000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000009
			6761766f66796f726b0000000000000000000000000000000000000000000000
		"
		);
		let string1 = Token::String("gavofyork".as_bytes().to_vec());
		let string2 = Token::String("gavofyork".as_bytes().to_vec());
		let tuple = Token::Tuple(vec![string1, string2]);
		let decoded = decode(
			&[ParamKind::Tuple(vec![Box::new(ParamKind::String), Box::new(ParamKind::String)])],
			&encoded,
		)
		.unwrap();
		let expected = vec![tuple];
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_nested_tuple() {
		let encoded = hex!(
			"
			0000000000000000000000000000000000000000000000000000000000000020
			0000000000000000000000000000000000000000000000000000000000000080
			0000000000000000000000000000000000000000000000000000000000000001
			00000000000000000000000000000000000000000000000000000000000000c0
			0000000000000000000000000000000000000000000000000000000000000100
			0000000000000000000000000000000000000000000000000000000000000004
			7465737400000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000006
			6379626f72670000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000060
			00000000000000000000000000000000000000000000000000000000000000a0
			00000000000000000000000000000000000000000000000000000000000000e0
			0000000000000000000000000000000000000000000000000000000000000005
			6e69676874000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000003
			6461790000000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000040
			0000000000000000000000000000000000000000000000000000000000000080
			0000000000000000000000000000000000000000000000000000000000000004
			7765656500000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000008
			66756e7465737473000000000000000000000000000000000000000000000000
		"
		);
		let string1 = Token::String("test".as_bytes().to_vec());
		let string2 = Token::String("cyborg".as_bytes().to_vec());
		let string3 = Token::String("night".as_bytes().to_vec());
		let string4 = Token::String("day".as_bytes().to_vec());
		let string5 = Token::String("weee".as_bytes().to_vec());
		let string6 = Token::String("funtests".as_bytes().to_vec());
		let bool = Token::Bool(true);
		let deep_tuple = Token::Tuple(vec![string5, string6]);
		let inner_tuple = Token::Tuple(vec![string3, string4, deep_tuple]);
		let outer_tuple = Token::Tuple(vec![string1, bool, string2, inner_tuple]);
		let expected = vec![outer_tuple];
		let decoded = decode(
			&[ParamKind::Tuple(vec![
				Box::new(ParamKind::String),
				Box::new(ParamKind::Bool),
				Box::new(ParamKind::String),
				Box::new(ParamKind::Tuple(vec![
					Box::new(ParamKind::String),
					Box::new(ParamKind::String),
					Box::new(ParamKind::Tuple(vec![
						Box::new(ParamKind::String),
						Box::new(ParamKind::String),
					])),
				])),
			])],
			&encoded,
		)
		.unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_complex_tuple_of_dynamic_and_static_types() {
		let encoded = hex!(
			"
			0000000000000000000000000000000000000000000000000000000000000020
			1111111111111111111111111111111111111111111111111111111111111111
			0000000000000000000000000000000000000000000000000000000000000080
			0000000000000000000000001111111111111111111111111111111111111111
			0000000000000000000000002222222222222222222222222222222222222222
			0000000000000000000000000000000000000000000000000000000000000009
			6761766f66796f726b0000000000000000000000000000000000000000000000
		"
		);
		let uint = Token::Uint([0x11u8; 32].into());
		let string = Token::String("gavofyork".as_bytes().to_vec());
		let address1 = Token::Address([0x11u8; 20].into());
		let address2 = Token::Address([0x22u8; 20].into());
		let tuple = Token::Tuple(vec![uint, string, address1, address2]);
		let expected = vec![tuple];
		let decoded = decode(
			&[ParamKind::Tuple(vec![
				Box::new(ParamKind::Uint(32)),
				Box::new(ParamKind::String),
				Box::new(ParamKind::Address),
				Box::new(ParamKind::Address),
			])],
			&encoded,
		)
		.unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_params_containing_dynamic_tuple() {
		let encoded = hex!(
			"
			0000000000000000000000002222222222222222222222222222222222222222
			00000000000000000000000000000000000000000000000000000000000000a0
			0000000000000000000000003333333333333333333333333333333333333333
			0000000000000000000000004444444444444444444444444444444444444444
			0000000000000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000001
			0000000000000000000000000000000000000000000000000000000000000060
			00000000000000000000000000000000000000000000000000000000000000a0
			0000000000000000000000000000000000000000000000000000000000000009
			7370616365736869700000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000006
			6379626f72670000000000000000000000000000000000000000000000000000
		"
		);
		let address1 = Token::Address([0x22u8; 20].into());
		let bool1 = Token::Bool(true);
		let string1 = Token::String("spaceship".as_bytes().to_vec());
		let string2 = Token::String("cyborg".as_bytes().to_vec());
		let tuple = Token::Tuple(vec![bool1, string1, string2]);
		let address2 = Token::Address([0x33u8; 20].into());
		let address3 = Token::Address([0x44u8; 20].into());
		let bool2 = Token::Bool(false);
		let expected = vec![address1, tuple, address2, address3, bool2];
		let decoded = decode(
			&[
				ParamKind::Address,
				ParamKind::Tuple(vec![
					Box::new(ParamKind::Bool),
					Box::new(ParamKind::String),
					Box::new(ParamKind::String),
				]),
				ParamKind::Address,
				ParamKind::Address,
				ParamKind::Bool,
			],
			&encoded,
		)
		.unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_params_containing_static_tuple() {
		let encoded = hex!(
			"
			0000000000000000000000001111111111111111111111111111111111111111
			0000000000000000000000002222222222222222222222222222222222222222
			0000000000000000000000000000000000000000000000000000000000000001
			0000000000000000000000000000000000000000000000000000000000000000
			0000000000000000000000003333333333333333333333333333333333333333
			0000000000000000000000004444444444444444444444444444444444444444
		"
		);
		let address1 = Token::Address([0x11u8; 20].into());
		let address2 = Token::Address([0x22u8; 20].into());
		let bool1 = Token::Bool(true);
		let bool2 = Token::Bool(false);
		let tuple = Token::Tuple(vec![address2, bool1, bool2]);
		let address3 = Token::Address([0x33u8; 20].into());
		let address4 = Token::Address([0x44u8; 20].into());

		let expected = vec![address1, tuple, address3, address4];
		let decoded = decode(
			&[
				ParamKind::Address,
				ParamKind::Tuple(vec![
					Box::new(ParamKind::Address),
					Box::new(ParamKind::Bool),
					Box::new(ParamKind::Bool),
				]),
				ParamKind::Address,
				ParamKind::Address,
			],
			&encoded,
		)
		.unwrap();
		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_fixed_array_of_strings() {
		// line 1 at 0x00 =   0: tail offset for the array
		// line 2 at 0x20 =  32: offset of string 1
		// line 3 at 0x40 =  64: offset of string 2
		// line 4 at 0x60 =  96: length of string 1
		// line 5 at 0x80 = 128: value  of string 1
		// line 6 at 0xa0 = 160: length of string 2
		// line 7 at 0xc0 = 192: value  of string 2
		let encoded = hex!(
			"
			0000000000000000000000000000000000000000000000000000000000000020
			0000000000000000000000000000000000000000000000000000000000000040
			0000000000000000000000000000000000000000000000000000000000000080
			0000000000000000000000000000000000000000000000000000000000000003
			666f6f0000000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000003
			6261720000000000000000000000000000000000000000000000000000000000
		"
		);

		let s1 = Token::String("foo".as_bytes().to_vec());
		let s2 = Token::String("bar".as_bytes().to_vec());
		let array = Token::FixedArray(vec![s1, s2]);

		let expected = vec![array];
		let decoded =
			decode(&[ParamKind::FixedArray(Box::new(ParamKind::String), 2)], &encoded).unwrap();

		assert_eq!(decoded, expected);
	}

	#[test]
	fn decode_after_fixed_bytes_with_less_than_32_bytes() {
		let encoded = hex!(
			"
			0000000000000000000000008497afefdc5ac170a664a231f6efb25526ef813f
			0000000000000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000000
			0000000000000000000000000000000000000000000000000000000000000080
			000000000000000000000000000000000000000000000000000000000000000a
			3078303030303030314600000000000000000000000000000000000000000000
		"
		);

		assert_eq!(
			decode(
				&[
					ParamKind::Address,
					ParamKind::FixedBytes(32),
					ParamKind::FixedBytes(4),
					ParamKind::String,
				],
				&encoded,
			)
			.unwrap(),
			&[
				Token::Address(hex!("8497afefdc5ac170a664a231f6efb25526ef813f").into()),
				Token::FixedBytes([0u8; 32].to_vec()),
				Token::FixedBytes([0u8; 4].to_vec()),
				Token::String("0x0000001F".into()),
			]
		);
	}
}
