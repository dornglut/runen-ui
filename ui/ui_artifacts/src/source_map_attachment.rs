use super::*;

pub(crate) fn push_attachment_entry(
    entries: &mut Vec<CompiledSourceMapEntry>,
    table: RuntimeTableKind,
    row: usize,
    attachment: Option<&UiProgramSourceMapAttachment>,
) {
    if let Some(attachment) = attachment {
        entries.push(CompiledSourceMapEntry::from_entry(
            &attachment.entry,
            table,
            row as u32,
            attachment.source_span,
            attachment.generated_by.to_owned(),
        ));
    }
}
