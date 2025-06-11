pub struct P2PNode {
    id: String,
    addr: SocketAddr,
    peer_manager: PeerManager,
    gossip_protocol: GossipProtocol,
    blockchain: Arc<Mutex<BlockchainState>>,
}

impl P2PNode {
    pub fn new(id: String, addr: SocketAddr, blockchain: Arc<Mutex<BlockchainState>>) -> Self {
        P2PNode {
            id,
            addr,
            peer_manager: PeerManager::new(),
            gossip_protocol: GossipProtocol::new(),
            blockchain,
        }
    }

    pub fn handle_message(&self, message: P2PMessage, peer_addr: SocketAddr) {
        if self.gossip_protocol.is_known(&message.validate().unwrap_or_default()) {
            return;
        }

        match message {
            P2PMessage::NewBlock(block) => {
                let mut blockchain = self.blockchain.lock().unwrap();
                if blockchain.add_block(block).is_ok() {
                    self.gossip_protocol.gossip_message(self, P2PMessage::NewBlock(block));
                }
            }
            P2PMessage::NewTransaction(transaction) => {
                self.blockchain.lock().unwrap().add_transaction(transaction.clone());
                self.gossip_protocol.gossip_message(self, P2PMessage::NewTransaction(transaction));
            }
            _ => {}
        }
    }
} 