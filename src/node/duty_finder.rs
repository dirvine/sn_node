// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use routing::Node as Routing;
use safe_nd::{Address, Cmd, DataCmd, Duty, ElderDuty, Message, MsgEnvelope, MsgSender, XorName};
use std::{cell::RefCell, rc::Rc};

#[allow(clippy::large_enum_variant)]
pub(crate) enum NodeDuties {
    Infant,
    Adult,
    Elder,
}

pub(crate) struct RemoteMsgEval {
    msg: MsgEnvelope,
    routing: Rc<RefCell<Routing>>,
    state: NodeDuties,
}

pub(crate) enum EvalOptions {
    ForwardToNetwork(MsgEnvelope),
    RunAtGateway(MsgEnvelope),
    RunAtPayment(MsgEnvelope),
    AccumulateForMetadata(MsgEnvelope),
    RunAtMetadata(MsgEnvelope),
    AccumulateForAdult(MsgEnvelope),
    RunAtAdult(MsgEnvelope),
    PushToClient(MsgEnvelope),
    RunAtRewards(MsgEnvelope),
    Unknown,
}

impl RemoteMsgEval {
    pub fn new(routing: Rc<RefCell<Routing>>) -> Self {
        Self { routing }
    }

    // todo: , duties: NodeDuties
    pub fn evaluate(&self, msg: MsgEnvelope) -> EvalOptions {
        if self.should_forward_to_network(msg) {
            // Any type of msg that is not process locally.
            EvalOptions::ForwardToNetwork(msg)
        } else if self.should_run_at_gateway() {
            // Client auth operations (Temporarily handled here, will be at app layer (Authenticator)).
            // Gateway Elders should just execute and return the result, for client to accumulate.
            EvalOptions::RunAtGateway(msg)
        } else if self.should_run_at_data_payment(msg) {
            // Incoming msg from `Gateway`!
            EvalOptions::RunAtPayment(msg) // Payment Elders should just execute and send onwards.
        } else if self.should_accumulate_for_metadata_write(msg) {
            // Incoming msg from `Payment`!
            EvalOptions::AccumulateForMetadata(msg) // Metadata Elders accumulate the msgs from Payment Elders.
        } else if self.should_run_at_metadata_write(msg) {
            // Accumulated msg from `Payment`!
            EvalOptions::RunAtMetadata(msg)
        } else if self.should_accumulate_for_chunk_write(msg) {
            // Incoming msg from `Metadata`!
            EvalOptions::AccumulateForAdult(msg) // Adults accumulate the msgs from Metadata Elders.
        } else if self.should_run_at_chunk_write(msg) {
            // Accumulated msg from `Metadata`!
            EvalOptions::RunAtAdult(msg)
        } else if self.should_push_to_client(msg) {
            // From network to client!
            EvalOptions::PushToClient(msg)
        } else if self.should_run_at_rewards() {
            EvalOptions::RunAtRewards(msg)
        } else {
            EvalOptions::Unknown
        }
    }

    fn should_forward_to_network(&self, msg: MsgEnvelope) -> bool {
        use Address::*;
        let destined_for_network = match msg.destination() {
            Client(_) => false,
            Node(address) => routing::XorName(address.0) != *self.routing.borrow().id().name(),
            Section(address) => !self.self_is_handler_for(&address),
        };
        let from_client = match msg.most_recent_sender() {
            MsgSender::Client { .. } => true,
            _ => false,
        };
        let is_auth_cmd = match msg.message {
            Message::Cmd {
                cmd: Cmd::Auth { .. },
                ..
            } => true,
            _ => false,
        };
        destined_for_network || (from_client && !is_auth_cmd)
    }

    // todo: eval all msg types!
    fn should_run_at_gateway(&self, msg: MsgEnvelope) -> bool {
        let from_client = match msg.most_recent_sender() {
            MsgSender::Client { .. } => true,
            _ => false,
        };
        let is_auth_cmd = match msg.message {
            Message::Cmd {
                cmd: Cmd::Auth { .. },
                ..
            } => true,
            _ => false,
        };
        // && we are Elder && self_is_handler_for(msg.destination())
        from_client && is_auth_cmd
    }

    /// We do not accumulate these request, they are executed
    /// at once (i.e. payment carried out) and sent on to
    /// Metadata section. (They however, will accumulate those msgs.)
    /// The reason for this is that the payment request is already signed
    /// by the client and validated by its replicas,
    /// so there is no reason to accumulate it here.
    fn should_run_at_data_payment(&self, msg: MsgEnvelope) -> bool {
        let from_gateway_elders = match msg.most_recent_sender() {
            MsgSender::Node {
                duty: Duty::Elder(ElderDuty::Gateway),
                ..
            } => true,
            _ => false,
        };
        let is_data_cmd = match msg.message {
            Message::Cmd {
                cmd: Cmd::Data { .. },
                ..
            } => true,
            _ => false,
        };
        // && we are Elder && self_is_handler_for(msg.destination())
        is_data_cmd && from_gateway_elders
    }

    /// The individual Payment Elder nodes send their msgs
    /// to Metadata section, where it is accumulated.
    fn should_accumulate_for_metadata_write(&self, msg: MsgEnvelope) -> bool {
        let from_payment_elder = match msg.most_recent_sender() {
            MsgSender::Node {
                duty: Duty::Elder(ElderDuty::Payment),
                ..
            } => true,
            _ => false,
        };
        let is_data_cmd = match msg.message {
            Message::Cmd {
                cmd: Cmd::Data { .. },
                ..
            } => true,
            _ => false,
        };
        // && we are Elder && self_is_handler_for(msg.destination())
        is_data_cmd && from_payment_elder
    }

    /// After the data write sent from Payment Elders has been
    /// accumulated (can be seen since the sender is `Section`),
    /// it is time to actually carry out the write operation.
    fn should_run_at_metadata_write(&self, msg: MsgEnvelope) -> bool {
        let from_payment_elders = match msg.most_recent_sender() {
            MsgSender::Section {
                duty: Duty::Elder(ElderDuty::Payment),
                ..
            } => true,
            _ => false,
        };
        let is_data_cmd = match msg.message {
            Message::Cmd {
                cmd: Cmd::Data { .. },
                ..
            } => true,
            _ => false,
        };
        // && we are Elder && self_is_handler_for(msg.destination())
        is_data_cmd && from_payment_elders
    }

    /// Adults accumulate the write requests from Elders.
    fn should_accumulate_for_chunk_write(&self, msg: MsgEnvelope) -> bool {
        let from_metadata_elders = match msg.most_recent_sender() {
            MsgSender::Node {
                duty: Duty::Elder(ElderDuty::Metadata),
                ..
            } => true,
            _ => false,
        };
        let is_data_cmd = match msg.message {
            Message::Cmd {
                cmd:
                    Cmd::Data {
                        cmd: DataCmd::Blob(_),
                        ..
                    },
                ..
            } => true,
            _ => false,
        };
        // && we are Adult && self_is_handler_for(msg.destination())
        is_data_cmd && from_metadata_elders
    }

    /// When the write requests from Elders has been accumulated
    /// at an Adult, it is time to carry out the write operation.
    fn should_run_at_chunk_write(&self, msg: MsgEnvelope) -> bool {
        let from_metadata_elders = match msg.most_recent_sender() {
            MsgSender::Section {
                duty: Duty::Elder(ElderDuty::Metadata),
                ..
            } => true,
            _ => false,
        };
        let is_data_cmd = match msg.message {
            Message::Cmd {
                cmd:
                    Cmd::Data {
                        cmd: DataCmd::Blob(_),
                        ..
                    },
                ..
            } => true,
            _ => false,
        };
        // && we are Adult && self_is_handler_for(msg.destination())
        is_data_cmd && from_metadata_elders
    }

    fn should_run_at_rewards(&self, msg: MsgEnvelope) -> bool {
        unimplemented!()
    }

    fn should_push_to_client(&self, msg: MsgEnvelope) -> bool {
        match msg.destination() {
            Address::Client(xorname) => self.self_is_handler_for(&xorname),
            _ => false,
        }
    }

    pub fn self_is_handler_for(&self, address: &XorName) -> bool {
        let xorname = routing::XorName(address.0);
        match self.routing.borrow().matches_our_prefix(&xorname) {
            Ok(result) => result,
            _ => false,
        }
    }
}