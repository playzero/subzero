<p align="center">
  <img src="https://zero.io/img/favicon.png" width="128">
</p>

<div align="center">
	[![GitLab Status](https://gitlab.io/zero.io/subzero/badges/master/pipeline.svg)](https://gitlab.io/zero.io/subzero/pipelines)
	[![GitHub tag (latest by date)](https://img.shields.io/github/v/tag/playzero/subzero)](https://github.com/playzero/subzero/tags)
	[![Substrate version](https://img.shields.io/badge/Substrate-2.0.0-brightgreen?logo=Parity%20Substrate)](https://substrate.dev/)
	[![License](https://img.shields.io/github/license/playzero/subzero?color=green)](https://github.com/playzero/subzero/blob/master/LICENSE)
	 <br />
	[![Discord](https://img.shields.io/badge/Discord-gray?logo=discord)](https://discord.gg/rhwtr7p)
	[![Twitter URL](https://img.shields.io/twitter/url?style=social&url=https%3A%2F%2Ftwitter.com%2Fzerodotio)](https://twitter.com/zerodotio)
	[![Medium](https://img.shields.io/badge/Medium-gray?logo=medium)](https://medium.com/playzero)
</div>

# 1. ZERO.IO — the videogame network for the metaverse

<!-- TOC -->

- [1. Introduction](#1-introduction)
- [2. Economics](#2-economics)
- [3. Build](#3-build)
- [4. Run](#4-run)

<!-- /TOC -->

# 1. Introduction
ZERO is a Relaychain, Parachain, Multiverse + DAO for next gen videogames. It provides native asset-, finance-, governance protocols, Smart Contracts, a Metaverse Hypergraph, Decentralized Computation at its core. To provide cross economic interoperability between totally different ecosystems. Futhermore it will enable generative creation of games and game economies through algorithms, generators and provision of autonomous agents to drive transition of old economy videogames and creators into a tokenized and decentralized future.

- Native currency: $PLAY (EUR backed)
- on/off ramp currencies:  $KSM, $DOT, FIAT, others t.b.d.
- join our community: http://discord.gg/rhwtr7p

# 2. Economics

ZERO Token ($PLAY) features the following utilities, and the value of $PLAY token will accrue with the increased usage of the network and revenue from stability fees and liquidation penalties

	- Network utility and stability fees
		-- Asset, Finance and Governance protocols
		-- Payment, Identity
		-- Computation, Oracles
	- Governance: vote for/against risk parameters and network change proposals
	- Economic Capital: in case of liquidation/defaulting of contracts without sufficient collaterals

To enable cross-chain functionality, ZERO.IO will connect to the Polkadot Ecosystem ( starting with Kusama ) in one of three ways:

	- parathread —— pay-as-you-go connection to Polkadot
	- parachain —— permanent connection for a given period
	- bridge —— independent chain bridged to Polkadot

Becoming a parachain would be an ideal option to bootstrap ZERO Network, to maximize its benefits and to reach to other chains and applications on the Polkadot network.

To secure a parachain slot, ZERO Network will require supportive DOT/KSM holders to lock their DOT/KSM to bid for a slot collectively — a process known as the Initial Parachain Offering (IPO). $PLAY tokens will be offered as a reward for those who participated in the IPO, as compensation for their opportunity cost of staking DOT/KSM.

3. Building
Rust.
```bash
	curl https://sh.rustup.rs -sSf | sh
```
Recursion for submodules in git
```bash
	git config --global submodule.recurse true
```
Build for your current machine architecture
```bash
	make build
```

4. Running
Run your local dev chain
```bash
	make run
```
Purge the cache
```bash
	make purge
```
Update ORML
```bash
	make update
```

# 0. Notes
 This is still work in progress, we will update more information as we progress. Refer to the token economy working paper for more details. This project and text was inspired by the excellent work of many growing projects in the Polkadot ecosystem. Thank you!.

