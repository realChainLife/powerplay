# PowerPlay Parachain

Parachains on Polkadot are extensible elements that can be plugged into the 
relay chain as validatable, globally coherent data structure. 

Powerplay is a cross-chain network for the exchange of functions like data or promises. 

## **Playing with the code**

To run and deploy your own instance of the Powerplay, ensure you have Rust installed. Here's how to get started: 

### Update Development Environment

    sudo apt-get update && apt-get upgrade -y
    sudo apt autoremove

### Install Rust & Wasm Compiler

    curl https://sh.rustup.rs -sSf | sh
    rustup install nightly
    rustup target add wasm32-unknown-unknown --toolchain nightly

### Install Support Software

    sudo apt install make clang pkg-config libssl-dev

### Build From Source 

    git clone https://github.com/realChainLife/powerplay
    cd powerplay
    cargo build
    cd collator

After you make changes in `src/lib.rs` & `collator/src/main.rs`, recompile the contract with:
    
    cargo build
    cargo run

Download wasm binaries for the parachain to the local machine:

    scp parachain:¬/powerplay/tests/res/powerplay.wasm ¬Downloads/adder.wasm --projects-development-225311

## **Deploying the Parachain**

Polkadot provides various JavaScript utilities and libraries for interacting with the network. The best way to deploy your parachain is through the [Polkadot Portal](https://polkadot.js.org/apps/#/explorer). 

- Choose a network to deploy to - Polkadot (Live, hosted by Web3 Foundation) is recommended. You can also deploy to a custom end-point. 
- Create your account and back up your keys (obviously)
- On the **extrinsic** tab submit the `registrar` function
- Select `registerPara(id, info, code, initial head data)`
- Add your `validationCode` and `HeadData`. Get this from your the powerplay.wasm file
- Finally, `Submit Transaction` for parachain to be registered. This will cost about 10 Dot. 



