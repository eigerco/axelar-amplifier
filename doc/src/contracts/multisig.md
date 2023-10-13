# Multisig contract

This contract is used by the prover contract during proof construction to start a signing session and collect signatures from participants.

```mermaid
flowchart TD
subgraph Axelar
  b[Prover]
  m[Multisig]
end
s[Signer worker]

b--StartSigningSession-->m
s--SubmitSignature-->m
b--GetSigningSession-->m
```

- **StartSigningSession**: The multisig contract receives a binary message from the prover contract. It uses the current active set of keys to link to a new signing session and then emits an event to notify signers that a message is pending signature.
- **SubmitSignature**: Each signer will sign the message using their own private key and then submit the signature to the multisig contract. This process validates that the signer is a participant in the snapshot associated with the active key that was set for the multisig session.
- **GetSigningSession**: Query called by the prover contract to retrieve the current state of the session, collected signatures so far and the snapshot with participants information. The proof is generated by the prover contract if the multisig was completed.

<br>

## Signing Sequence Diagram

```mermaid
sequenceDiagram
participant Relayer
box Axelar
participant Prover
participant Multisig
end
actor Signers


Relayer->>+Prover: ExecuteMsg::ConstructProof
Prover->>+Multisig: ExecuteMsg::StartSigningSession
Multisig-->>Signers: emit SigningStarted event
Multisig->>-Prover: reply with session ID
deactivate Prover
loop Collect signatures
  Signers->>+Multisig: ExecuteMsg::SubmitSignature
  Multisig-->>Relayer: emit SignatureSubmitted event
end
Multisig-->>-Relayer: emit SigningCompleted event
Relayer->>+Prover: QueryMsg::GetProof
Prover->>+Multisig: QueryMsg::GetSigningSession
Multisig-->>-Prover: reply with status, current signatures vector and snapshot
Prover-->>-Relayer: returns data and proof

```

## Interface

```Rust
pub enum ExecuteMsg {
    StartSigningSession {
        msg: HexBinary,
    },
    SubmitSignature {
        session_id: Uint64,
        signature: HexBinary,
    },
}

#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetSigningSessionResponse)]
    GetSigningSession { session_id: Uint64 },
}

pub struct GetSigningSessionResponse {
    state: MultisigState,
    signatures: HashMap<String, HexBinary>,
    snapshot: Snapshot,
}

pub enum MultisigState {
    Pending,
    Completed,
}
```

## Events

```Rust
pub enum Event {
    // Emitted when a new signing session is open
    SigningStarted {
        session_id: Uint64,
        key_id: Uint64,
        pub_keys: HashMap<String, HexBinary>,
        msg: HexBinary,
    },
    // Emitted when a participants submits a signature
    SignatureSubmitted {
        session_id: Uint64,
        participant: Addr,
        signature: HexBinary,
    },
    // Emitted when a signing session was completed
    SigningCompleted {
        session_id: Uint64,
    },
}
```