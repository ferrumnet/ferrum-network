use ferrum_x_runtime::{
    AccountId, AuraConfig, BalancesConfig, EVMConfig, EthereumConfig, GenesisConfig, GrandpaConfig,
    Signature, SudoConfig, SystemConfig, WASM_BINARY, //QuantumPortalConfig
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public, H160, U256};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    AccountId32,
};
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};

use crate::{
    cli::Cli,
    config::{convert, Config, NetworkConfig},
};

const DEFAULT_DEV_PATH_BUF: &str = "./default_dev_config.json";
const DEFAULT_LOCAL_TESTNET_PATH_BUF: &str = "./default_dev_config.json";

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn config_path_buf(cli: &Cli, dev: bool) -> PathBuf {
    if let Some(local_path_buf) = cli.run.config_file_path.clone() {
        local_path_buf
    } else if dev {
        PathBuf::from(DEFAULT_DEV_PATH_BUF)
    } else {
        PathBuf::from(DEFAULT_LOCAL_TESTNET_PATH_BUF)
    }
}

pub fn config_elem(cli: &Cli, dev: bool) -> Result<Config, String> {
    let path_buf = config_path_buf(cli, dev);

    crate::config::read_config_from_file(path_buf)
}

pub fn chainspec_params(
    config_elem: Config,
) -> Result<
    (
        Vec<(AuraId, GrandpaId)>,
        AccountId,
        Vec<AccountId>,
        Vec<String>,
    ),
    String,
> {
    let chain_spec_config = config_elem.chain_spec;
    let address_list = chain_spec_config.address_list.clone();
    let initial_authoutities: Vec<_> = chain_spec_config
        .initial_authourity_seed_list
        .into_iter()
        .map(|seed| authority_keys_from_seed(&seed))
        .collect();
    let root_key = AccountId::from_str(chain_spec_config.root_seed.as_str()).unwrap();
    let endowed_accounts: Vec<AccountId> = chain_spec_config
        .endowed_accounts_seed_list
        .into_iter()
        .map(|seed| AccountId::from_str(chain_spec_config.root_seed.as_str()).unwrap())
        .collect();

    Ok((
        initial_authoutities,
        root_key,
        endowed_accounts,
        address_list,
    ))
}

pub fn development_config(cli: &Cli) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    let config_elem = config_elem(cli, true)?;

    let networks = config_elem.networks.clone();

    let (initial_authoutities, root_key, endowed_accounts, address_list) =
        chainspec_params(config_elem)?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                initial_authoutities.clone(),
                // Sudo account
                root_key.clone(),
                // Pre-funded accounts
                endowed_accounts.clone(),
                address_list.clone(),
                networks.clone(),
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

pub fn local_testnet_config(cli: &Cli) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    let config_elem = config_elem(cli, false)?;

    let networks = config_elem.networks.clone();

    let (initial_authoutities, root_key, endowed_accounts, address_list) =
        chainspec_params(config_elem)?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                initial_authoutities.clone(),
                // Sudo account
                root_key.clone(),
                // Pre-funded accounts
                endowed_accounts.clone(),
                address_list.clone(),
                networks.clone(),
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    address_list: Vec<String>,
    networks: NetworkConfig,
    _enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        },
        aura: AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        },
        grandpa: GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        },
        sudo: SudoConfig {
            // Assign network admin rights.
            key: Some(root_key),
        },
        evm: EVMConfig {
            accounts: {
                let map: BTreeMap<_, fp_evm::GenesisAccount> = address_list
                    .into_iter()
                    .map(|address| {
                        (
                            H160::from_str(address.as_str()).expect("internal H160 is valid; qed"),
                            fp_evm::GenesisAccount {
                                balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
                                    .expect("internal U256 is valid; qed"),
                                code: Default::default(),
                                nonce: Default::default(),
                                storage: Default::default(),
                            },
                        )
                    })
                    .collect();
                map
            },
        },
        ethereum: EthereumConfig {},
        dynamic_fee: Default::default(),
        base_fee: Default::default(),
        // quantum_portal: QuantumPortalConfig {
        //     networks: convert(networks),
        // },
    }
}
