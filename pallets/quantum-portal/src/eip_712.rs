//! EIP712 structs
use serde_json::{Value};
use std::collections::HashMap;
use ethereum_types::{U256, H256, Address};
use regex::Regex;
use validator::Validate;
use validator::ValidationErrors;
use lazy_static::lazy_static;

pub(crate) type MessageTypes = HashMap<String, Vec<FieldType>>;

lazy_static! {
	// match solidity identifier with the addition of '[(\d)*]*'
	static ref TYPE_REGEX: Regex = Regex::new(r"^[a-zA-Z_$][a-zA-Z_$0-9]*(\[([1-9]\d*)*\])*$").unwrap();
	static ref IDENT_REGEX: Regex = Regex::new(r"^[a-zA-Z_$][a-zA-Z_$0-9]*$").unwrap();
}

#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[derive(Deserialize, Serialize, Validate, Debug, Clone)]
pub(crate) struct EIP712Domain {
	pub(crate) name: String,
	pub(crate) version: String,
	pub(crate) chain_id: U256,
	pub(crate) verifying_contract: Address,
	#[serde(skip_serializing_if="Option::is_none")]
	pub(crate) salt: Option<H256>,
}
/// EIP-712 struct
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[derive(Deserialize, Debug, Clone)]
pub struct EIP712 {
	pub(crate) types: MessageTypes,
	pub(crate) primary_type: String,
	pub(crate) message: Value,
	pub(crate) domain: EIP712Domain,
}

impl Validate for EIP712 {
	fn validate(&self) -> Result<(), ValidationErrors> {
		for field_types in self.types.values() {
			for field_type in field_types {
				field_type.validate()?;
			}
		}
		Ok(())
	}
}

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
pub(crate) struct FieldType {
	#[validate(regex = "IDENT_REGEX")]
	pub name: String,
	#[serde(rename = "type")]
	#[validate(regex = "TYPE_REGEX")]
	pub type_: String,
}

fn check_hex(string: &str) -> Result<()> {
	if string.len() >= 2 && &string[..2] == "0x" {
		return Ok(())
	}

	return Err(ErrorKind::HexParseError(
		format!("Expected a 0x-prefixed string of even length, found {} length string", string.len()))
	)?
}
/// given a type and HashMap<String, Vec<FieldType>>
/// returns a HashSet of dependent types of the given type
fn build_dependencies<'a>(message_type: &'a str, message_types: &'a MessageTypes) -> Option<(HashSet<&'a str>)>
{
	if message_types.get(message_type).is_none() {
		return None;
	}

	let mut types = IndexSet::new();
	types.insert(message_type);
	let mut deps = HashSet::new();

	while let Some(item) = types.pop() {
		if let Some(fields) = message_types.get(item) {
			deps.insert(item);

			for field in fields {
				// seen this type before? or not a custom type skip
				if deps.contains(&*field.type_) || !message_types.contains_key(&*field.type_) {
					continue;
				}
				types.insert(&*field.type_);
			}
		}
	};

	return Some(deps)
}

fn encode_type(message_type: &str, message_types: &MessageTypes) -> Result<String> {
	let deps = {
		let mut temp = build_dependencies(message_type, message_types).ok_or_else(|| ErrorKind::NonExistentType)?;
		temp.remove(message_type);
		let mut temp = temp.into_iter().collect::<Vec<_>>();
		(&mut temp[..]).sort_unstable();
		temp.insert(0, message_type);
		temp
	};

	let encoded = deps
		.into_iter()
		.filter_map(|dep| {
			message_types.get(dep).map(|field_types| {
				let types = field_types
					.iter()
					.map(|value| format!("{} {}", value.type_, value.name))
					.join(",");
				return format!("{}({})", dep, types);
			})
		})
		.collect::<Vec<_>>()
		.concat();
	Ok(encoded)
}

fn type_hash(message_type: &str, typed_data: &MessageTypes) -> Result<H256> {
	Ok(keccak(encode_type(message_type, typed_data)?))
}

fn encode_data(
	parser: &Parser,
	message_type: &Type,
	message_types: &MessageTypes,
	value: &Value,
	field_name: Option<&str>
) -> Result<Vec<u8>>
{
	let encoded = match message_type {
		Type::Array {
			inner,
			length
		} => {
			let mut items = vec![];
			let values = value.as_array().ok_or_else(|| serde_error("array", field_name))?;

			// check if the type definition actually matches
			// the length of items to be encoded
			if length.is_some() && Some(values.len() as u64) != *length {
				let array_type = format!("{}[{}]", *inner, length.unwrap());
				return Err(ErrorKind::UnequalArrayItems(length.unwrap(), array_type, values.len() as u64))?
			}

			for item in values {
				let mut encoded = encode_data(parser, &*inner, &message_types, item, field_name)?;
				items.append(&mut encoded);
			}

			keccak(items).to_vec()
		}

		Type::Custom(ref ident) if message_types.get(&*ident).is_some() => {
			let type_hash = (&type_hash(ident, &message_types)?).to_vec();
			let mut tokens = encode(&[EthAbiToken::FixedBytes(type_hash)]);

			for field in message_types.get(ident).expect("Already checked in match guard; qed") {
				let value = &value[&field.name];
				let type_ = parser.parse_type(&*field.type_)?;
				let mut encoded = encode_data(parser, &type_, &message_types, &value, Some(&*field.name))?;
				tokens.append(&mut encoded);
			}

			keccak(tokens).to_vec()
		}

		Type::Bytes => {
			let string = value.as_str().ok_or_else(|| serde_error("string", field_name))?;

			check_hex(&string)?;

			let bytes = (&string[2..])
				.from_hex::<Vec<u8>>()
				.map_err(|err| ErrorKind::HexParseError(format!("{}", err)))?;
			let bytes = keccak(&bytes).to_vec();

			encode(&[EthAbiToken::FixedBytes(bytes)])
		}

		Type::Byte(_) => {
			let string = value.as_str().ok_or_else(|| serde_error("string", field_name))?;

			check_hex(&string)?;

			let bytes = (&string[2..])
				.from_hex::<Vec<u8>>()
				.map_err(|err| ErrorKind::HexParseError(format!("{}", err)))?;

			encode(&[EthAbiToken::FixedBytes(bytes)])
		}

		Type::String => {
			let value = value.as_str().ok_or_else(|| serde_error("string", field_name))?;
			let hash = keccak(value).to_vec();
			encode(&[EthAbiToken::FixedBytes(hash)])
		}

		Type::Bool => encode(&[EthAbiToken::Bool(value.as_bool().ok_or_else(|| serde_error("bool", field_name))?)]),

		Type::Address => {
			let addr = value.as_str().ok_or_else(|| serde_error("string", field_name))?;
			if addr.len() != 42 {
				return Err(ErrorKind::InvalidAddressLength(addr.len()))?;
			}
			let address = EthAddress::from_str(&addr[2..]).map_err(|err| ErrorKind::HexParseError(format!("{}", err)))?;
			encode(&[EthAbiToken::Address(address)])
		}

		Type::Uint | Type::Int => {
			let string = value.as_str().ok_or_else(|| serde_error("int/uint", field_name))?;

			check_hex(&string)?;

			let uint = U256::from_str(&string[2..]).map_err(|err| ErrorKind::HexParseError(format!("{}", err)))?;

			let token = if *message_type == Type::Uint {
				EthAbiToken::Uint(uint)
			} else {
				EthAbiToken::Int(uint)
			};
			encode(&[token])
		}

		_ => return Err(ErrorKind::UnknownType(format!("{}", field_name.unwrap_or("")), format!("{}", *message_type)))?
	};

	Ok(encoded)
}

/// encodes and hashes the given EIP712 struct
pub fn hash_structured_data(typed_data: EIP712) -> Result<H256> {
	// validate input
	typed_data.validate()?;
	// EIP-191 compliant
	let prefix = (b"\x19\x01").to_vec();
	let domain = to_value(&typed_data.domain).unwrap();
	let parser = Parser::new();
	let (domain_hash, data_hash) = (
		encode_data(&parser, &Type::Custom("EIP712Domain".into()), &typed_data.types, &domain, None)?,
		encode_data(&parser, &Type::Custom(typed_data.primary_type), &typed_data.types, &typed_data.message, None)?
	);
	let concat = [&prefix[..], &domain_hash[..], &data_hash[..]].concat();
	Ok(keccak(concat))
}