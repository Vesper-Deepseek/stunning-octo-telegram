use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use std::path::PathBuf;

fn main() {
    // Provide the path to your local GGUF model.
    // Set via the LOOSE_ENDS_MODEL_PATH environment variable or replace
    // the placeholder below with the actual path to your GGUF model file.
    let model_path = "REPLACE_WITH_YOUR_GGUF_MODEL_PATH";
    let backend = LlamaBackend::init().unwrap();
    let model_params = LlamaModelParams::default();
    let model =
        LlamaModel::load_from_file(&backend, PathBuf::from(model_path), &model_params).unwrap();

    let grammar = include_str!("extraction_grammar.gbnf");
    println!("Grammar length: {}", grammar.len());
    println!("First 200 chars: {}", &grammar[..200.min(grammar.len())]);

    let sampler = LlamaSampler::grammar(&model, grammar, "root");
    match sampler {
        Ok(_) => println!("Grammar parsed successfully!"),
        Err(e) => eprintln!("Grammar error: {:?}", e),
    }
}
