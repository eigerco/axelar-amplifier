# Aleo ITS-Hub and Axelar ITS-Hub message translation

# ITS HubMessage

ITS HubMesage defines 3 messages.

1. `SendToHub`
1. `ReceiveFromHub`
1. `RgisterTokenMetadata`

`SendToHub` and `ReceiveFromHub` define a direction for the message, they contain an ITS Message and the destination or the origin chain respectively.

# ITS Message

| Axelar ITS-Hub        | Direction | Aleo ITS-Hub                    |
| --------------------- | :-------: | ------------------------------- |
| InterchainTransfer    |    ->     | IncomingInterchainTransfer      |
| InterchainTransfer    |    <-     | OutgoingInterchainTransfer      |
| DeployInterchainToken |    ->     | FromRemoteDeployInterchainToken |
| DeployInterchainToken |    <-     | DeployInterchainToken           |
| LinkToken             |    <->    | TBD                             |

->: ReceiveFromHub
<-: SendToHub

# Aleo ITS Messages

| Axelar ITS-Hub                        | Aleo ITS-Hub                                                     |
| ------------------------------------- | ---------------------------------------------------------------- |
| SendToHub(InterchainTransfer)         | ItsIncomingInterchainTransfer                                    |
| ReceiveFromHub(InterchainTransfer)    | ItsOutgoingInterchainTransfer                                    |
| SendToHub(DeployInterchainToken)      | RemoteDeployInterchainToken(DeployInterchainToken)               |
| ReceiveFromHub(DeployInterchainToken) | ItsMessageDeployInterchainToken(FromRemoteDeployInterchainToken) |
| RgisterTokenMetadata                  | TBD                                                              |

| Check list | Rust impl                                                                 | Axelar ITS-Hub        | Direction | Aleo ITS-Hub                    |
| :--------: | :------------------------------------------------------------------------ | --------------------- | :-------: | ------------------------------- |
|    [ ]     | `impl TryFrom<InterchainTransfer> for IncomingInterchainTransfer`         | InterchainTransfer    |    ->     | IncomingInterchainTransfer      |
|    [ ]     | `impl TryFrom<OutgoingInterchainTransfer> for InterchainTransfer`         | InterchainTransfer    |    <-     | OutgoingInterchainTransfer      |
|    [ ]     | `impl TryFrom<DeployInterchainToken> for FromRemoteDeployInterchainToken` | DeployInterchainToken |    ->     | FromRemoteDeployInterchainToken |
|    [ ]     | `impl TryFrom<Aleo::DeployInterchainToken> for DeployInterchainToken`     | DeployInterchainToken |    <-     | DeployInterchainToken           |

```mermaid
flowchart LR
    subgraph EVM
        A[📤 SendToHub<br/>InterchainTransfer<br/>destination: Aleo]
        H[📥 ReceiveFromHub<br/>InterchainTransfer<br/>source: Aleo]
        L[🚀 DeployInterchainToken<br/>destination: Aleo]
        M[📥 ReceiveFromHub<br/>DeployInterchainToken<br/>source: Aleo]
    end
    subgraph Aleo
        E[📥 ReceiveFromHub<br/>IncomingInterchainTransfer<br/>source: EVM]
        G[📤 SendToHub<br/>ItsOutgoingInterchainTransfer<br/>OutgoingInterchainTransfer<br/>destination: EVM]
        N[📥 ReceiveFromHub<br/>ItsMessageDeployInterchainToken<br/>FromRemoteDeployInterchainToken<br/>source: EVM]
        O[🚀 SendToHub<br/>RemoteDeployInterchainToken<br/>DeployInterchainToken <br/>destination: EVM]
    end
    subgraph ITSHub
        C[🔄 TranslateFromAbiToHubMessage]
        F[⚙️ Hub Message<br/>Processing]
        D[🔄 TranslateFromHubMessageToAleo]
        I[🔄 TranslateFromAleoToHubMessage]
        J[⚙️ Hub Message<br/>Processing]
        K[🔄 TranslateFromHubMessageToAbi]
    end
    %% EVM to Aleo flow
    A --> C
    C --> F
    F --> D
    D --> E
    %% Aleo to EVM flow
    G --> I
    I --> J
    J --> K
    K --> H
    %% EVM DeployInterchainToken to Aleo flow
    L --> C
    D --> N
    %% Aleo DeployInterchainToken to EVM flow
    O --> I
    K --> M
    style EVM fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    style A fill:#e1f5fe
    style H fill:#e1f5fe
    style L fill:#c8e6c9
    style M fill:#c8e6c9
    style F fill:#ffeb3b
    style J fill:#ffeb3b
    style ITSHub fill:#f0f0f0,stroke:#333,stroke-width:2px
    style E fill:#e8f5e8
    style G fill:#e8f5e8
    style N fill:#c8e6c9
    style O fill:#c8e6c9
    style Aleo fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
```
