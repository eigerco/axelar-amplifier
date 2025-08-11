use leo_codegen::generate_leo_structs_pretty;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("generated_structs.rs");

    let leo_code = r#"
        program foobar.aleo {
            struct DeployInterchainToken {
                its_token_id: [u128; 2],
                name: u128,
                symbol: u128,
                decimals: u8,
                minter: [u128; 6],
            }

            // From the remote chain to Aleo
            struct FromRemoteDeployInterchainToken {
                its_token_id: [u128; 2],
                name: u128,
                symbol: u128,
                decimals: u8,
                minter: address,                    // the minter address, if there is no minter the zero address should be used
            }

            // This is the payload that will be sent to the Axelar ITS-hub
            struct RemoteDeployInterchainToken {
                payload: DeployInterchainToken,
                destination_chain: [u128; 2],
            }

            struct IncomingInterchainTransfer {
                its_token_id: [u128; 2],
                source_address: [u128; 6],
                destination_address: address,       // the address that the token will be sent
                amount: u128,
            }

            struct OutgoingInterchainTransfer {
                its_token_id: [u128; 2],
                source_address: address,
                destination_address: [u128; 6],
                amount: u128,
            }

            struct ItsOutgoingInterchainTransfer {
                inner_message: OutgoingInterchainTransfer,
                destination_chain: [u128; 2],
            }

            // A remote chain want to deploy/bridge a token on Aleo
            struct ItsMessageDeployInterchainToken {
                inner_message: FromRemoteDeployInterchainToken,
                source_chain: [u128; 2]
            }

            struct ItsIncomingInterchainTransfer {
                inner_message: IncomingInterchainTransfer,
                source_chain: [u128; 2],
            }
        }
    "#;

    generate_leo_structs_pretty(&dest_path, "testnet", leo_code)?;

    Ok(())
}
