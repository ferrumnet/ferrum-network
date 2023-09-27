# Ferrum Network Docs

Ferrum Network enables value, data, and functional interoperability between every blockchain in the industry.

## Interoperability by Design
Ferrum Network's sophisticated solutions simplify the complexity of building a multichain solution and give developers and project owners a single entry point to every recognizable chain and network in the industry. We have thought about the pain points that arise when implementing interoperability as an afterthought in a multi-network / multichain world. With an ever-growing list of new EVM and non-EVM compatible chains coming to market, gaining traction, and providing value to segments of the global crypto audience, interoperability has become a core value for most projects. At Ferrum Network, we design from the ground up for interoperability.

You can read more about Ferrum Network and its mission [here](https://docs.ferrumnetwork.io/ferrum-network-ecosystem/introduction/overview)

This documentation is focused on setting up the ferrum node, running miner and finaliser nodes, and developing applications that use the Quantum Portal for cross chain communication.

For learning more about Ferrum network and Quantum portal, you can use the links below : 
 
- [About Ferrum Network](https://docs.ferrumnetwork.io/ferrum-network-ecosystem/introduction/overview)
- [Core Tech](https://docs.ferrumnetwork.io/ferrum-network-ecosystem/architecture/core-tech)
- [Whitepaper](https://docs.ferrumnetwork.io/ferrum-network-ecosystem/architecture/core-tech/quantum-portal)


## What is Ferrum Network

Ferrum network is an evm-compatible substrate parachain. A parachain is an application-specific data structure that is globally coherent and can be validated by the validators of the Relay Chain. They take their name from the concept of parallelized chains that run parallel to the Relay Chain.

### Why Parachains?

Parachains are a solution to two fundamental problems in blockchains:

Scalability: Having one blockchain for many purposes makes it difficult to scale as future implementations and upgrades will likely advantage some purposes and disadvantage others. On the other hand, having different blockchains will allow them to implement features themselves without affecting other chains.
Flexibility: It is reasonable to state a blockchain either will be really good in solving one problem or not so good trying to solve many problems. A blockchain able to specialize in solving a specific problem has more leverage towards itself and its users. Parachains are purpose-built blockchains highly specialized and able to take advantage from each other by cooperation.

You can read more about Parachains here: https://wiki.polkadot.network/docs/learn-parachains