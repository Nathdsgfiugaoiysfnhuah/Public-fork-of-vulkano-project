use std::time::{SystemTime, UNIX_EPOCH};

pub fn do_fps(frames: &mut [f64], cur_frame: &mut u32, lt: &mut f64){
    *cur_frame += 1;
    *cur_frame %= 15;
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as f64;
    // println!("{since_the_epoch:?}");
    frames[*cur_frame as usize] = since_the_epoch / 1000f64 - *lt;
    *lt = since_the_epoch / 1000f64;
    let mut sum_time = 0f64;
    for frame_time in frames.iter() {
        sum_time += frame_time;
    }
    sum_time /= 15f64;
    let fps = 1f64 / sum_time + 0.5;
    if *cur_frame%15==0 {print!("\rFPS: {fps:.0?}   ")};
}
