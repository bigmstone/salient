use std::{
    error::Error,
    fmt::{Display, Error as FmtError, Formatter},
};

#[derive(Debug, PartialEq)]
enum FilterValue {
    Int(i32),
    Str(String),
}

#[derive(Debug, PartialEq)]
struct Condition {
    column: String,
    filter: FilterValue,
}

#[derive(Debug, PartialEq)]
enum GroupFunctions {
    TimeBucket(i32),
}

#[derive(Debug, PartialEq)]
enum AggregationFunction {
    Count,
}

#[derive(Debug, PartialEq)]
struct Aggregation {
    function: AggregationFunction,
    group_by: Option<GroupFunctions>,
}

#[derive(Debug, PartialEq)]
enum Operation {
    Filter(Vec<Condition>),
    Group(Aggregation),
}

#[derive(Debug, PartialEq)]
struct OperationNode {
    operation: Operation,
    child: Option<Box<OperationNode>>, // Children are directly managed via Box
}

#[derive(Debug, PartialEq)]
pub struct Query {
    pub root: OperationNode,
}

impl Query {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let mut root = None;

        let parts: Vec<&str> = input.split('|').map(str::trim).collect();
        for part in parts {
            println!("Part: {}", part);
            let operation = if let Some(condition_str) = part.strip_prefix("filter") {
                let conditions = Self::parse_conditions(condition_str.trim())?;
                Operation::Filter(conditions)
            } else if let Some(aggregation_str) = part.strip_prefix("group") {
                println!("Attempting group of: {}", aggregation_str.trim());
                let aggregation = Self::parse_aggregation(aggregation_str.trim())?;
                Operation::Group(aggregation)
            } else {
                return Err(ParseError::new("Unsupported operation"));
            };

            let new_node = Box::new(OperationNode {
                operation,
                child: None,
            });

            if root.is_none() {
                root = Some(new_node);
            } else {
                let mut current_node = root
                    .as_mut()
                    .ok_or_else(|| ParseError::new("Value didn't exist that should"))?;
                for i in 0..1000 {
                    if current_node.child.is_some() {
                        current_node = current_node
                            .child
                            .as_mut()
                            .ok_or_else(|| ParseError::new("Value didn't exist that should"))?;
                    } else {
                        current_node.child = Some(new_node);
                        break;
                    }
                    if i == 999 {
                        return Err(ParseError::new("Exceeded query operator limit"));
                    }
                }
            }
        }

        root.map(|node| Query { root: *node })
            .ok_or_else(|| ParseError::new(""))
    }

    fn parse_conditions(input: &str) -> Result<Vec<Condition>, ParseError> {
        let mut conditions = Vec::new();
        for part in input.split("AND").map(str::trim).filter(|s| !s.is_empty()) {
            let parts: Vec<&str> = part.split('=').map(str::trim).collect();
            if parts.len() != 2 {
                return Err(ParseError::new(
                    "Each condition must contain exactly one '=' character",
                ));
            }
            let column = parts[0].to_string();
            let value_str = parts[1];
            let filter = Self::parse_filter_value(value_str)?;
            conditions.push(Condition { column, filter });
        }
        Ok(conditions)
    }

    fn parse_aggregation(input: &str) -> Result<Aggregation, ParseError> {
        let parts = input
            .split("BY")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();

        let mut aggregation = Aggregation {
            function: AggregationFunction::Count,
            group_by: None,
        };

        if parts[0].contains("count()") {
            aggregation.function = AggregationFunction::Count;
        } else {
            return Err(ParseError::new(&format!(
                "Unsupported group function: {}",
                parts[0]
            )));
        }

        if let Some(start) = input.find("timebucket(") {
            let end = input[start..].find(')').ok_or(ParseError::new(
                "Missing closing parenthesis for timebucket",
            ))? + start;
            let args = &input[start + "timebucket(".len()..end];
            let size = args
                .trim()
                .parse::<i32>()
                .map_err(|_| ParseError::new("Invalid bucket size"))?;
            aggregation.group_by = Some(GroupFunctions::TimeBucket(size));
        } else {
            return Err(ParseError::new("Unsupported aggregation function"));
        }

        Ok(aggregation)
    }

    fn parse_filter_value(value_str: &str) -> Result<FilterValue, ParseError> {
        if value_str.starts_with('\'') && value_str.ends_with('\'') && value_str.len() > 1 {
            Ok(FilterValue::Str(
                value_str[1..value_str.len() - 1].to_string(),
            ))
        } else {
            value_str.parse::<i32>().map(FilterValue::Int).or_else(|_| {
                Err(ParseError::new(
                    "String values must be enclosed in single quotes",
                ))
            })
        }
    }
}

#[derive(Debug)]
struct ParseError {
    context: String,
}

impl ParseError {
    pub fn new(context: &str) -> Self {
        Self {
            context: context.to_string(),
        }
    }
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FmtError> {
        formatter.write_str(&self.context)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_filter() {
        let test_values = vec![
            (
                Query::parse("filter bob = 'test' AND sally = 'tester' AND jeff = 33").unwrap(),
                Query {
                    root: OperationNode {
                        operation: Operation::Filter(vec![
                            Condition {
                                column: "bob".to_string(),
                                filter: FilterValue::Str("test".to_string()),
                            },
                            Condition {
                                column: "sally".to_string(),
                                filter: FilterValue::Str("tester".to_string()),
                            },
                            Condition {
                                column: "jeff".to_string(),
                                filter: FilterValue::Int(33),
                            },
                        ]),
                        child: None,
                    },
                },
            ),
            (
                Query::parse("filter bob = 'test' AND jeff = 33 AND sally = 'tester'").unwrap(),
                Query {
                    root: OperationNode {
                        operation: Operation::Filter(vec![
                            Condition {
                                column: "bob".to_string(),
                                filter: FilterValue::Str("test".to_string()),
                            },
                            Condition {
                                column: "jeff".to_string(),
                                filter: FilterValue::Int(33),
                            },
                            Condition {
                                column: "sally".to_string(),
                                filter: FilterValue::Str("tester".to_string()),
                            },
                        ]),
                        child: None,
                    },
                },
            ),
        ];

        for (query, query_ds) in test_values {
            assert_eq!(query, query_ds);
        }
    }

    #[test]
    fn test_aggregation() {
        let test_values = vec![(
            Query::parse(
                "filter bob = 'test' AND sally = 'tester' AND jeff = 33 | group count() by timebucket(1)",
            )
            .unwrap(),
            Query {
                root: OperationNode {
                    operation: Operation::Filter(vec![
                        Condition {
                            column: "bob".to_string(),
                            filter: FilterValue::Str("test".to_string()),
                        },
                        Condition {
                            column: "sally".to_string(),
                            filter: FilterValue::Str("tester".to_string()),
                        },
                        Condition {
                            column: "jeff".to_string(),
                            filter: FilterValue::Int(33),
                        },
                    ]),
                    child: Some(Box::new(OperationNode {
                        operation: Operation::Group(Aggregation {
                            function: AggregationFunction::Count,
                            group_by: Some(GroupFunctions::TimeBucket(1)),
                        }),
                        child: None,

                    })),
                },
            },
        )];

        for (query, query_ds) in test_values {
            assert_eq!(query, query_ds);
        }
    }

    #[test]
    fn test_invalid_values() {
        let queries = vec![
            "filter bob = 'test' AND sally = 'tester' AND jeff = 3.2",
            "filter bob = 'test' AND sally = tester AND jeff = 32",
            "filter bob = test AND sally = 'tester' AND jeff = 32",
        ];

        for query in queries {
            if Query::parse(query).is_ok() {
                panic!("Query parsing should have failed: {}", query);
            }
        }
    }

    #[test]
    fn test_parse_aggregation() {
        let agg = Query::parse_aggregation("count() by timebucket(1)").unwrap();

        assert_eq!(
            agg,
            Aggregation {
                function: AggregationFunction::Count,
                group_by: Some(GroupFunctions::TimeBucket(1)),
            }
        );
    }
}
