use chrono::{DateTime, FixedOffset};


fn pattern_from_char(ch: char) -> u8 {
    if ch == ' ' {
        0
    } else {
        let ch = ch as u32 - 0x2800;
        
        let mut rv = (ch & 0b0011_1000) << 1 | (ch & 0b0000_0111);
        if ch & 0x0040 > 0 {
            rv |= 0b0000_1000;
        }
        if ch & 0x0080 > 0 {
            rv |= 0b1000_0000;
        }
        rv as u8
    }
}

fn char_for_pattern(pattern: u8) -> char {
    if pattern == 0 {
        ' '
    } else {
        let rv = (pattern & 0b0111_0000) >> 1 | (pattern & 0b0000_0111);
        let mut rv = rv as u32 + 0x2800;
        if pattern & 0b0000_1000 > 0 {
            rv += 0x40;
        }
        if pattern & 0b1000_0000 > 0 {
            rv += 0x80;
        }
        char::from_u32(rv).expect("Should always be in range")
    }
}

fn plot_at(pattern: u8, buf: &mut char) {
    let pattern = pattern | pattern_from_char(*buf);
    let ch = char_for_pattern(pattern);
    *buf = ch;
}

#[test]
fn test_pattern_for() {
    assert_eq!(0b1000, pattern_for(0, 0));
    assert_eq!(0b0100, pattern_for(1, 1));
    assert_eq!(0b0010, pattern_for(2, 2));
    assert_eq!(0b0001, pattern_for(3, 3));

    assert_eq!(0b1100, pattern_for(0, 1));
    assert_eq!(0b1110, pattern_for(0, 2));
    assert_eq!(0b0110, pattern_for(1, 2));
    assert_eq!(0b0111, pattern_for(1, 5));
    assert_eq!(0b1111, pattern_for(0, 5));
    assert_eq!(0b1100, pattern_for(5, 1));
    for i in 0u8.. {
        let mut ch = ' ';
        plot_at(i, &mut ch);
        print!("{ch}");
    }
}
fn pattern_for(from: i64, to: i64) -> u8 {
    let mut pattern = 1 << (3 - (from % 4));
    pattern = if to / 4 == from / 4 {
        pattern | (1 << (3 - (to % 4)))
    } else if to < from {
        pattern | 0b1000
    } else {
        pattern | 0b0001
    };
    if pattern == 0b101 {
        pattern = 0b111;
    }
    if pattern == 0b1010 {
        pattern = 0b1110;
    }
    if pattern == 0b1001 {
        pattern = 0b1111;
    }
    pattern
}
fn y_at(from: (i64, i64), to: (i64, i64), x: i64) -> i64 {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    from.1 + (dy * (x - from.0)) / dx
}
#[test]
fn test_y_at() {
    assert_eq!(y_at((0, 0), (10, 10), 0), 0);
    assert_eq!(y_at((0, 0), (10, 10), 10), 10);
    assert_eq!(y_at((0, 0), (10, 10), 5), 5);
    assert_eq!(y_at((0, 10), (10, 0), 0), 10);
    assert_eq!(y_at((0, 10), (10, 0), 10), 0);
    assert_eq!(y_at((0, 10), (10, 0), 5), 5);
    assert_eq!(y_at((102, 15), (136, 20), 120), 17);
}
fn plot_line(from: (i64, i64), to: (i64, i64), buf: &mut [Vec<char>]) {
    let mut pt = from;

    if from.0 == to.0 {
        // Vertical line
        while (from.1 < to.1 && pt.1 <= to.1) || (from.1 > to.1 && pt.1 >= to.1){
            let mut pattern = pattern_for(pt.1, to.1);
            if pt.0 % 2 == 1 {
                pattern <<= 4;
            }
            
            
            plot_at(pattern, &mut buf[buf.len() - 1 - pt.1 as usize / 4][pt.0 as usize / 2]);
            if to.1 > from.1 {
                pt.1 -= pt.1 % 4;
                pt.1 += 4;                
            } else {
                pt.1 += 3 - pt.1 % 4;
                pt.1 -= 4;
                
            }
        }
    } else {
        
        while pt.0 < to.0 {
            let mut pattern = pattern_for(pt.1, y_at(from, to, pt.0 + 1));
            if pt.0 % 2 == 1 {
                pattern <<= 4;
            }
            plot_at(pattern, &mut buf[buf.len() - 1 - pt.1 as usize / 4][pt.0 as usize / 2]);
            pt.0 += 1;
            pt.1 = y_at(from, to, pt.0);
        }
    }
}

pub fn plot_times(label: &str, width: usize, height: usize, times: &[DateTime<FixedOffset>]) {
    let times: Vec<_> = times.iter().map(|dt| dt.time()).collect();
    let min = *times.iter().min().unwrap();
    let max = *times.iter().max().unwrap();
    let duration = max - min;
    let row_height = duration / height as i32;
    let pt_height = row_height / 4;
    let mut buf = vec![vec![' '; width]; height + 1];
    let horiz_size = width as f32  / times.len() as f32;
    for (i, times) in times.windows(2).enumerate() {
        let y1_pt = (times[0] - min).num_milliseconds() / pt_height.num_milliseconds();
        let y2_pt = (times[1] - min).num_milliseconds() / pt_height.num_milliseconds();
        
        plot_line(((i as f32 * horiz_size) as i64 * 2, y1_pt), (((i + 1) as f32 * horiz_size) as i64 * 2, y2_pt), &mut buf);
        
    }
    for (i, row) in buf.into_iter().enumerate() {
        let row_tag = if i == 1 {
            max.format("%H:%M:%S").to_string()
        } else if i == height {
            min.format("%H:%M:%S").to_string()
        } else if i == height / 2 {
            label.to_string()
        } else {
            "".to_string()
        };
        println!("{:>10} {}", row_tag, row.into_iter().collect::<String>());
    }
}
