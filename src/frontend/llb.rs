use buildkit_llb::{
    ops::{FileSystem, MultiOwnedLastOutput, OperationBuilder, SingleOwnedOutput, Source},
    prelude::LayerPath,
    utils::{OperationOutput, OutputIdx, OwnOutputIdx},
};
use dofigen_lib::Dofigen;

pub trait ToLlbOperationOutput {
    fn to_llb_operation_output(&self) -> OperationOutput<'static>;
}

impl ToLlbOperationOutput for Dofigen {
    fn to_llb_operation_output(&self) -> OperationOutput<'static> {
        let context = Source::local("context");
        let context = self.context.iter().fold(context, |context, pattern| {
            eprintln!("Adding include pattern: {}", pattern);
            context.add_include_pattern(pattern.clone())
        });
        let context = self.ignore.iter().fold(context, |context, pattern| {
            eprintln!("Adding exclude pattern: {}", pattern);
            context.add_exclude_pattern(pattern.clone())
        });
        let context = context.ref_counted();

        // TODO: implement based on Dofigen data
        
        // FROM alpine
        let from = Source::image("alpine:latest").ref_counted();
        let mut sequence = FileSystem::sequence(); //.custom_name("Runtime fs actions");
        // WORKDIR /app
        let workdir = FileSystem::mkdir(OutputIdx(0), LayerPath::Other(from.output(), "/app"))
            .make_parents(true);
        sequence = sequence.append(workdir);
        let last_index = sequence.last_output_index().clone().unwrap();
        // COPY logo.svg .
        let copy = FileSystem::copy()
            .from(LayerPath::Other(context.output(), "logo.svg"))
            .to(
                OutputIdx(last_index + 1),
                LayerPath::Own(OwnOutputIdx(last_index), "/app/"),
            )
            // Optional: create the destination path if it doesn't exist
            .create_path(true);
        sequence = sequence.append(copy);
        // RUN echo coucou
        // TODO: implement `RUN` operation and add it to the sequence.

        sequence.ref_counted().last_output().unwrap()
    }
}
