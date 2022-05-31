const Web3 = require('web3');

const { poc_bytecode, poc_abi } = require('./poc_consts');
const { ldg_mgr_bytecode, ldg_mgr_abi } = require('./ldg_mgr_consts');

// Initialization

const privKey =
   '99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342'; // Genesis private key
const address = '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b';
const web3 = new Web3('http://localhost:9933');
// Deploy contract
async function deploy(bytecode, abi) {
   console.log('Attempting to deploy from account:', address);
const quantumPortalPoc = new web3.eth.Contract(abi);
const quantumPortalPocTx = quantumPortalPoc.deploy({
      data: bytecode,
   });
let createTransaction = await web3.eth.accounts.signTransaction(
      {
         from: address,
         data: quantumPortalPocTx.encodeABI(),
         gas: '4294967',
      },
      privKey
   );
let createReceipt = await web3.eth.sendSignedTransaction(
      createTransaction.rawTransaction
   );
   console.log('Contract deployed at address', createReceipt.contractAddress);
};

const all_deploy = async () => {
  await deploy(poc_bytecode, poc_abi);
  await deploy(ldg_mgr_bytecode, ldg_mgr_abi);
}

all_deploy();
