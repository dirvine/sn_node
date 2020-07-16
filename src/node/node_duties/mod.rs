// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

mod network_events;
mod msg_analysis;
mod accumulation;
mod messaging;

use network_events::NetworkEvents;
use msg_analysis::{NodeOperation, NetworkMsgAnalysis};
use accumulation::Accumulation;
use messaging::{Messaging, Receiver, Received};
use crate::{
    accumulation::Accumulation,
    cmd::{GroupDecision, MessagingDuty},
    node::{
        adult_duties::AdultDuties,
        elder_duties::ElderDuties,
        keys::NodeKeys,
    },
    Config, Result,
};
use routing::Node as Routing;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[allow(clippy::large_enum_variant)]
enum AgeBasedDuties {
    Infant,
    Adult(AdultDuties),
    Elder(ElderDuties),
}

struct NodeDuties {
    keys: NodeKeys
    age_based: AgeBasedDuties,
    network_events: NetworkEvents,
    accumulation: Accumulation,
    messaging: Messaging,
    routing: RefCell<Rc<Routing>>,
    config: Config,
}

impl NodeDuties {
    
    pub fn new(keys: NodeKeys, age_based: AgeBasedDuties,
        routing: RefCell<Rc<Routing>>, config: Config) -> Self {
        let accumulation = Accumulation::new(routing.clone());
        let network_events = NetworkEvents::new(NetworkMsgAnalysis::new(routing.clone()));
        let messaging = Messaging::new(routing.clone());
        Self {
            config,
            age_based,
            network_events,
            accumulator,
            messaging
            routing,
        }
    }

    pub fn process(&mut self, duty: NodeDuty) -> Option<NodeOperation> {
        use NodeDuty::*;
        match duty {
            BecomeAdult => self.become_adult(),
            BecomeElder => self.become_elder(),
            Accumulate(msg) => self.accumulation.process(&msg),
            ProcessMessaging(duty) => self.messaging.process(duty),
            ProcessNetworkEvent(event) => self.network_events.process(event),
        }
    }

    fn become_adult(&mut self) -> Result<()> {
        use AgeBasedDuties::*;
        // let mut config = Config::default();
        // config.set_root_dir(self.root_dir.clone());
        let total_used_space = Rc::new(Cell::new(0));
        self.age_based = Adult(AdultDuties::new(self.keys.clone(), &self.config, &total_used_space, Init::New)?);
        Ok(())
    }

    fn become_elder(&mut self) -> Result<()> {
        use AgeBasedDuties::*;
        // let mut config = Config::default();
        // config.set_root_dir(self.root_dir.clone());
        let total_used_space = Rc::new(Cell::new(0));

        input_parsing: InputParsing,
        section: SectionQuerying,
        client_msg_tracking: ClientMsgTracking,

        let duties = ElderDuties::new(
            self.keys.clone(),
            &self.config,
            &total_used_space,
            Init::New,
            routing.clone(),
            ClientMessaging::new(self.id.public_id().clone(), routing),
        )?;
        self.age_based = Elder(duties);
        Ok(())
    }
}
