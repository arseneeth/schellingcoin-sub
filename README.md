# Schelling Coin: Minimal-Trust Data Feed Protocol in Substrate

<img src="https://s3.amazonaws.com/ngccoin-production/world-coin-price-guide/85381f.jpg" height="150" width="150">

In 2014 Vitalik Buterin [proposed](https://blog.ethereum.org/2014/03/28/schellingcoin-a-minimal-trust-universal-data-feed/) and [prototyped](https://blog.ethereum.org/2014/06/30/advanced-contract-programming-example-schellingcoin/) one of the first implementations of Decentralized Data Feed smart contracts for Ethereum blockchain. 

This project is an attempt to build a PoC of Schelling Coin protocol in Substrate. There are several motivations behind it. The first one is to demonstrate that the implementation of Schelling coin in runtime logic rather than smart contract logic might make the protocol more scalable and fast. The second one is to initiate the discussion around Substrate based decentralized data-feed systems that might be beneficial for emerging decentralized systems such as Polkadot and DOthereum.

# Protocol Mechanics

Here we provide a rather mechanistic explanation, for more game-theoretic motivations behind the algorithm please check out the original [blogpost](https://blog.ethereum.org/2014/03/28/schellingcoin-a-minimal-trust-universal-data-feed/). 

The general idea behind the protocol is that everyone “votes” on a particular value and everyone who submitted a vote that is between the 25th and 75 percentile (ie. close to median) receives a reward.

The basic protocol steps are as follows:

1. During the first half of the epoch, users submit the hash of their address together with the value that they "vote" and "locks" some amount of tokens as a deposit.
2. During the second half of the epoch, users submit the value whose has they provided in the first half of the epoch.
3. Hash the value provided and the user address in order to compare it with the hash from the first half of the epoch.
4. If hashes match add values to the list and sort it
5. Everybody who submitted values between 25th and 75th percentile receive their stake back and a reward. Those who didn't get into the range receive their stake with a small decrease as a penalty.   

<img src="https://blog.ethereum.org/wp-content/uploads/2014/11/schellingcoin.png" height="400" width="400">


# Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
```

Build the WebAssembly binary:

```bash
./scripts/build.sh
```

Build all native code:

```bash
cargo build
```

# Run

You can start a development chain with:

```bash
cargo run -- --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units. Give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet). You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.
