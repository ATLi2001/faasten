use crate::syscalls;
use labeled::dclabel::{Clause, Component, DCLabel};

pub fn proto_label_to_dc_label(label: syscalls::DcLabel) -> DCLabel {
    DCLabel {
        secrecy: match label.secrecy {
            None => Component::DCFalse,
            Some(set) => Component::DCFormula(
                set.clauses
                    .iter()
                    .map(|c| {
                        Clause(c.principals.iter().map(Clone::clone).collect())
                    })
                    .collect(),
            ),
        },
        integrity: match label.integrity {
            None => Component::DCFalse,
            Some(set) => Component::DCFormula(
                set.clauses
                    .iter()
                    .map(|c| {
                        Clause(c.principals.iter().map(Clone::clone).collect())
                    })
                    .collect(),
            ),
        },
    }
}

pub fn dc_label_to_proto_label(label: &DCLabel) -> syscalls::DcLabel {
    syscalls::DcLabel {
        secrecy: match &label.secrecy {
            Component::DCFalse => None,
            Component::DCFormula(set) => Some(syscalls::Component {
                clauses: set
                    .iter()
                    .map(|clause| syscalls::Clause {
                        principals: clause.0.iter().map(Clone::clone).collect(),
                    })
                    .collect(),
            }),
        },
        integrity: match &label.integrity {
            Component::DCFalse => None,
            Component::DCFormula(set) => Some(syscalls::Component {
                clauses: set
                    .iter()
                    .map(|clause| syscalls::Clause {
                        principals: clause.0.iter().map(Clone::clone).collect(),
                    })
                    .collect(),
            }),
        },
    }
}