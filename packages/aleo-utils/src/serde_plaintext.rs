//! - Serializes Rust types into aleo plain text strings with [`to_string`]
//! - Deserializes aleo plain text strings into Rust types with [`from_str`]
//!
//! Serialization/Deserialization is done without any regard for aleo-specific
//! types (like Address, Signature, Field, Group, and Scalar), so it's only
//! useful when one does not need to make use of such Aleo-specific types as
//! can be the case during data structure transformations (i.e. from one Rust
//! type to another).
//!
//! If an use case demands the conversion to Aleo's Plaintext Rust type
//! (i.e for hashing) one can take the result of [`to_string`] and construct
//! the Plaintext Rust type from it

mod de;
mod error;
mod parser;
mod ser;

pub use de::from_str;
pub use ser::to_string;

#[cfg(test)]
mod test {

    use serde::{Deserialize, Serialize};

    use crate::serde_plaintext::{self};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub struct ApproveMessagesExecuteData {
        pub proof: Proof,
        pub message: MessageWrapper,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub struct ApproveMessagesInputs {
        pub weighted_signer: WeightedSigners,
        pub signatures: [[String; 14]; 2],
        pub message: [[String; 24]; 2],
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub struct Proof {
        pub weighted_signer: WeightedSigners,
        pub signatures: [[String; 14]; 2],
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub struct WeightedSigners {
        pub signers: [[WeightedSigner; 14]; 2],
        pub quorum: u128,
        // nonce: [u64; 4], // TODO: this should be included before going to main net
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub struct WeightedSigner {
        pub addr: String,
        pub weight: u128,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub struct MessageWrapper {
        pub messages: Vec<Message>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub struct Message {
        pub source_chain: [u128; 2],
        pub message_id: [u128; 8],
        pub source_address: [u128; 6],
        pub contract_address: String,
        pub payload_hash: String,
    }

    #[test]
    fn test_deserialize_approve_messages_execute_data() {
        let expected = ApproveMessagesExecuteData {
            proof: Proof {
                weighted_signer: WeightedSigners {
                    signers: [
                        [
                            WeightedSigner {addr: "aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak".into(), weight: 1},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0}
                        ],
                        [
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0},
                            WeightedSigner {addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0}
                        ],
                    ],
                    quorum: 1,
                },
                signatures: [
                    [
                        "sign14jtzy97tktas4mhcm3hx6qynzw38t0sf8uz8p9zng63rtygvtupfwuru9mk7ykjeurkt9x0xrpy9q05qnaa0dgpymc7mkaw2recqjqvpxp6apcrgnzaw3gfdywla9m4vxywvhnsuewd38alswp3pxmu8zt6pj54ng8w2txvnzjp5c3pyu9lt54f6hlgxuln98jgzrwnpsqassdml9ya".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into()
                    ],
                    [
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                        "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into()
                    ],
                ],
            },
            message: MessageWrapper {
                messages: vec![Message {
                    source_chain: [129497940983541880690546129557858025472, 0],
                    message_id: [
                        129543616418537684110010247973178472295,
                        146813217023299466576067115680920779129,
                        154488831008045460057537329038304032872,
                        69646294136802280241651158794438180864,
                        0,
                        0,
                        0,
                        0
                    ],
                    source_address: [
                        129497940984858643952261301375189137254,
                        141528452179676227430456616297872650616,
                        151824918195357812520057881859406328177,
                        158438911409234173347848527060049097728,
                        0,
                        0,
                    ],
                    contract_address: "aleo1lklakkgnjv6a9m0kyjhm33f66xql8n2pj4lgsd2emhep0cwe8uqqqcdups".into(),
                    payload_hash: "4063574237844910514730141909505057216530945707718220555768700040578731323345group".into()
                }],
            },
        };

        let execute_data = include_str!("../test_data/approve_messages_execute_data.plaintext");
        let actual: ApproveMessagesExecuteData = serde_plaintext::from_str(execute_data).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_serialize_approve_messages_aleo_inputs() {
        let approve_messages_inputs = ApproveMessagesInputs {
            weighted_signer: WeightedSigners {
                signers: [
                    [
                        WeightedSigner { addr: "aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak".into(), weight: 1u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 }
                    ],
                    [
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 },
                        WeightedSigner { addr: "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc".into(), weight: 0u128 }
                    ]
                ],
                quorum: 1u128
            },
            signatures: [
                [
                    "sign14jtzy97tktas4mhcm3hx6qynzw38t0sf8uz8p9zng63rtygvtupfwuru9mk7ykjeurkt9x0xrpy9q05qnaa0dgpymc7mkaw2recqjqvpxp6apcrgnzaw3gfdywla9m4vxywvhnsuewd38alswp3pxmu8zt6pj54ng8w2txvnzjp5c3pyu9lt54f6hlgxuln98jgzrwnpsqassdml9ya".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into()
                ],
                [
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into(),
                    "sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml".into()
                ]
            ],
            message: [
                [
                    "6774940913874822158876702936744450268717023155196858268815542658027655866696group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                ],
                [
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                    "0group".into(),
                ]
            ]
        };

        let expected= "{weighted_signer:{signers:[[{addr:aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak,weight:1u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128}],[{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128},{addr:aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc,weight:0u128}]],quorum:1u128},signatures:[[sign14jtzy97tktas4mhcm3hx6qynzw38t0sf8uz8p9zng63rtygvtupfwuru9mk7ykjeurkt9x0xrpy9q05qnaa0dgpymc7mkaw2recqjqvpxp6apcrgnzaw3gfdywla9m4vxywvhnsuewd38alswp3pxmu8zt6pj54ng8w2txvnzjp5c3pyu9lt54f6hlgxuln98jgzrwnpsqassdml9ya,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml],[sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml,sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml]],message:[[6774940913874822158876702936744450268717023155196858268815542658027655866696group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group],[0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group,0group]]}";
        let actual = serde_plaintext::to_string(&approve_messages_inputs).unwrap();
        assert_eq!(actual, expected);
    }
}
