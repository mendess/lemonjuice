// use async_std::{task, stream::{self, Stream}};
// use futures_util::StreamExt;
// use super::{Block,Config};
// use crate::text::Text;

// pub fn make_block_stream(config: Config) -> impl Stream<Item = Text> {
//     stream::empty()
// }

// pub fn block_to_stream(mut block: Block) -> impl Stream<Item = Option<Text>> {
//     stream::repeat(()).then(move |_| async {
//         task::sleep(block.interval).await;
//         block.update();
//         block.to_text(0)
//     })
// }
