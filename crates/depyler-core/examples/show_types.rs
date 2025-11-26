//! Demo: Show inferred types from dataflow analysis

use depyler_core::{dataflow::DataflowTypeInferencer, DepylerPipeline};

fn main() -> anyhow::Result<()> {
    let python_code = r#"
def calculate_stats(numbers: list[int]) -> dict[str, float]:
    total = 0
    count = 0
    
    for num in numbers:
        total = total + num
        count = count + 1
    
    if count > 0:
        average = total / count
    else:
        average = 0.0
    
    result = {"sum": total, "count": count, "average": average}
    return result

def process_names(names: list[str]) -> list[str]:
    upper_names = []
    for name in names:
        upper = name.upper()
        upper_names.append(upper)
    return upper_names

def find_max(items):
    if len(items) == 0:
        return None
    
    max_val = items[0]
    for item in items:
        if item > max_val:
            max_val = item
    return max_val
"#;

    let pipeline = DepylerPipeline::new();
    let hir = pipeline.parse_to_hir(python_code)?;
    
    println!("=== Dataflow Type Inference Results ===\n");
    
    let inferencer = DataflowTypeInferencer::new();
    
    for func in &hir.functions {
        println!("Function: {}", func.name);
        println!("{}", "-".repeat(40));
        
        let result = inferencer.infer_function(func);
        
        // Show parameter types
        println!("  Parameters:");
        for param in &func.params {
            println!("    {}: {:?}", param.name, param.ty);
        }
        
        // Show inferred variable types
        println!("  Variables:");
        for (var, ty) in result.variable_types.iter() {
            println!("    {}: {:?}", var, ty);
        }
        
        // Show return type
        if let Some(ret_ty) = &result.return_type {
            println!("  Return type: {:?}", ret_ty);
        }
        
        println!("  Iterations to fixpoint: {}", result.iterations);
        println!("  Complete inference: {}", result.is_complete);
        println!();
    }
    
    Ok(())
}
