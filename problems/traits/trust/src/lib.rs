#![forbid(unsafe_code)]

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundOutcome {
    BothCooperated,
    LeftCheated,
    RightCheated,
    BothCheated,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PersonalOutcome {
    Cooperate,
    Cheat,
}

pub trait Agent {
    fn act(&self) -> PersonalOutcome;
    fn memorise_op_act(&mut self, _op_act: PersonalOutcome) {}
}

pub struct Game {
    left_ag: Box<dyn Agent>,
    right_ag: Box<dyn Agent>,
    total_left_score: i32,
    total_right_score: i32,
}

impl Game {
    pub fn new(left: Box<dyn Agent>, right: Box<dyn Agent>) -> Self {
        Self {
            left_ag: left,
            right_ag: right,
            total_left_score: 0,
            total_right_score: 0,
        }
    }

    pub fn left_score(&self) -> i32 {
        self.total_left_score
    }

    pub fn right_score(&self) -> i32 {
        self.total_right_score
    }

    pub fn play_round(&mut self) -> RoundOutcome {
        let (left_out, right_out) = (self.left_ag.act(), self.right_ag.act());
        let round_outcome = match (&left_out, &right_out) {
            (PersonalOutcome::Cooperate, PersonalOutcome::Cooperate) => {
                self.total_left_score += 2;
                self.total_right_score += 2;
                RoundOutcome::BothCooperated
            }
            (PersonalOutcome::Cooperate, PersonalOutcome::Cheat) => {
                self.total_left_score += -1;
                self.total_right_score += 3;
                RoundOutcome::RightCheated
            }
            (PersonalOutcome::Cheat, PersonalOutcome::Cooperate) => {
                self.total_left_score += 3;
                self.total_right_score += -1;
                RoundOutcome::LeftCheated
            }
            (PersonalOutcome::Cheat, PersonalOutcome::Cheat) => RoundOutcome::BothCheated,
        };

        self.left_ag.memorise_op_act(right_out);
        self.right_ag.memorise_op_act(left_out);

        round_outcome
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct CheatingAgent {}

impl Agent for CheatingAgent {
    fn act(&self) -> PersonalOutcome {
        PersonalOutcome::Cheat
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct CooperatingAgent {}

impl Agent for CooperatingAgent {
    fn act(&self) -> PersonalOutcome {
        PersonalOutcome::Cooperate
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct GrudgerAgent {
    opinion: PersonalOutcome,
}

impl Agent for GrudgerAgent {
    fn act(&self) -> PersonalOutcome {
        self.opinion
    }

    fn memorise_op_act(&mut self, op_act: PersonalOutcome) {
        if op_act == PersonalOutcome::Cheat {
            self.opinion = PersonalOutcome::Cheat;
        }
    }
}

impl Default for GrudgerAgent {
    fn default() -> Self {
        Self {
            opinion: PersonalOutcome::Cooperate,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct CopycatAgent {
    prev_op_act: PersonalOutcome,
}

impl Agent for CopycatAgent {
    fn act(&self) -> PersonalOutcome {
        self.prev_op_act
    }

    fn memorise_op_act(&mut self, op_act: PersonalOutcome) {
        self.prev_op_act = op_act;
    }
}

impl Default for CopycatAgent {
    fn default() -> Self {
        Self {
            prev_op_act: PersonalOutcome::Cooperate,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct DetectiveAgent {
    rounds_played: u8,
    as_copycat: bool,
    prev_op_act: PersonalOutcome,
}

impl Agent for DetectiveAgent {
    fn act(&self) -> PersonalOutcome {
        match self.rounds_played {
            0 => PersonalOutcome::Cooperate,
            1 => PersonalOutcome::Cheat,
            2 => PersonalOutcome::Cooperate,
            3 => PersonalOutcome::Cooperate,
            _ => self.prev_op_act,
        }
    }

    fn memorise_op_act(&mut self, op_act: PersonalOutcome) {
        if self.rounds_played <= 3 && op_act == PersonalOutcome::Cheat {
            self.as_copycat = true;
        }
        if self.rounds_played >= 3 && self.as_copycat {
            self.prev_op_act = op_act;
        }
        self.rounds_played = self.rounds_played.saturating_add(1);
    }
}

impl Default for DetectiveAgent {
    fn default() -> Self {
        Self {
            rounds_played: 0,
            as_copycat: false,
            prev_op_act: PersonalOutcome::Cheat,
        }
    }
}

// TODO: your code goes here.
