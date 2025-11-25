//! Contract definitions and checking

use depyler_core::hir::{HirFunction, Type};
use serde::{Deserialize, Serialize};

use super::contract_verification::{InvariantChecker, PostconditionVerifier, PreconditionChecker, VerificationResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub preconditions: Vec<Condition>,
    pub postconditions: Vec<Condition>,
    pub invariants: Vec<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub name: String,
    pub expression: String,
    pub description: String,
}

pub struct ContractChecker;

impl ContractChecker {
    pub fn extract_contracts(func: &HirFunction) -> Contract {
        let mut contract = Contract {
            preconditions: vec![],
            postconditions: vec![],
            invariants: vec![],
        };

        Self::extract_docstring_into_contract(&mut contract, &func.docstring);
        Self::extract_param_preconditions(&mut contract, &func.params);
        Self::extract_return_postconditions(&mut contract, &func.ret_type);
        Self::extract_property_invariants(&mut contract, &func.properties);

        contract
    }

    fn extract_docstring_into_contract(contract: &mut Contract, docstring: &Option<String>) {
        if let Some(doc) = docstring {
            let extracted = Self::extract_docstring_contracts(doc);
            contract.preconditions.extend(extracted.preconditions);
            contract.postconditions.extend(extracted.postconditions);
            contract.invariants.extend(extracted.invariants);
        }
    }

    fn extract_param_preconditions(contract: &mut Contract, params: &[depyler_core::hir::HirParam]) {
        for param in params {
            if let Type::List(_) = &param.ty {
                contract.preconditions.push(Condition {
                    name: format!("{}_not_null", param.name),
                    expression: format!("{} is not None", param.name),
                    description: format!("Parameter {} must not be null", param.name),
                });
            }
        }
    }

    fn extract_return_postconditions(contract: &mut Contract, ret_type: &Type) {
        match ret_type {
            Type::Optional(_) => {
                contract.postconditions.push(Condition {
                    name: "result_valid".to_string(),
                    expression: "result is None or result meets type constraints".to_string(),
                    description: "Result must be None or valid value".to_string(),
                });
            }
            Type::List(_) => {
                contract.postconditions.push(Condition {
                    name: "result_not_null".to_string(),
                    expression: "result is not None".to_string(),
                    description: "Result list must not be null".to_string(),
                });
            }
            _ => {}
        }
    }

    fn extract_property_invariants(contract: &mut Contract, properties: &depyler_core::hir::FunctionProperties) {
        if properties.panic_free {
            contract.invariants.push(Condition {
                name: "no_panics".to_string(),
                expression: "all array accesses are bounds-checked".to_string(),
                description: "Function must not panic on any input".to_string(),
            });
        }

        if properties.always_terminates {
            contract.invariants.push(Condition {
                name: "termination".to_string(),
                expression: "loop variants decrease monotonically".to_string(),
                description: "Function must terminate for all inputs".to_string(),
            });
        }

        if properties.is_pure {
            contract.invariants.push(Condition {
                name: "purity".to_string(),
                expression: "no side effects".to_string(),
                description: "Function must not have observable side effects".to_string(),
            });
        }
    }

    fn extract_docstring_contracts(docstring: &str) -> Contract {
        let mut contract = Contract {
            preconditions: vec![],
            postconditions: vec![],
            invariants: vec![],
        };

        for line in docstring.lines() {
            let trimmed = line.trim();

            if let Some(pre) = trimmed.strip_prefix("Requires:") {
                contract.preconditions.push(Condition {
                    name: "docstring_precondition".to_string(),
                    expression: pre.trim().to_string(),
                    description: "From docstring".to_string(),
                });
            } else if let Some(post) = trimmed.strip_prefix("Ensures:") {
                contract.postconditions.push(Condition {
                    name: "docstring_postcondition".to_string(),
                    expression: post.trim().to_string(),
                    description: "From docstring".to_string(),
                });
            } else if let Some(inv) = trimmed.strip_prefix("Invariant:") {
                contract.invariants.push(Condition {
                    name: "docstring_invariant".to_string(),
                    expression: inv.trim().to_string(),
                    description: "From docstring".to_string(),
                });
            }
        }

        contract
    }

    pub fn verify_contracts(func: &HirFunction, contract: &Contract) -> Vec<VerificationResult> {
        let mut results = Vec::new();

        // Check preconditions
        let pre_checker = PreconditionChecker::new();
        for pre in &contract.preconditions {
            results.push(pre_checker.check(func, pre));
        }

        // Check postconditions
        let post_verifier = PostconditionVerifier::new();
        for post in &contract.postconditions {
            results.push(post_verifier.verify(func, post));
        }

        // Check invariants
        let inv_checker = InvariantChecker::new();
        for inv in &contract.invariants {
            results.push(inv_checker.check(func, inv));
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use depyler_core::hir::Type;
    use smallvec::smallvec;

    #[test]
    fn test_extract_contracts() {
        let func = HirFunction {
            name: "test".to_string(),
            params: smallvec![],
            ret_type: Type::Int,
            body: vec![],
            properties: Default::default(),
            annotations: Default::default(),
            docstring: Some("Requires: x > 0\nEnsures: result >= 0".to_string()),
        };

        let contract = ContractChecker::extract_contracts(&func);
        assert!(!contract.preconditions.is_empty());
        assert!(!contract.postconditions.is_empty());
    }
}
