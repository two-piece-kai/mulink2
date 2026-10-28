use num::Complex;
use plotters::{
    chart::ChartBuilder,
    prelude::{BitMapBackend, DiscreteRanged, IntoDrawingArea, IntoLinspace, PathElement},
    series::LineSeries,
    style::{RED, WHITE},
};

use crate::prelude::SignalType;

pub fn plot<T: SignalType + Clone>(filename: &str, label: &str, values: &[T]) {
    let values: Vec<f64> = values.iter().cloned().map(|x| x.as_f64()).collect();
    let root_area = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();
    root_area.fill(&WHITE).unwrap();
    let x_axis = (0 as f64..values.len() as f64).step(1.0);

    let x_space = 0.0..values.len() as f64;
    let y_min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    let y_span = y_max - y_min;
    let (y_max, y_min) = (y_min + y_span * 1.2, y_max - y_span * 1.2);
    //let y_min = y_max - y_span * 1.2;

    let y_space = y_min..y_max;

    //println!("processing span for plot");

    let mut cc = ChartBuilder::on(&root_area)
        .margin(5)
        .set_all_label_area_size(50)
        .caption(label, ("sans-serif", 40))
        .build_cartesian_2d(x_space, y_space)
        .unwrap();

    //println!("processing X0 for plot");

    cc.configure_mesh()
        .x_labels(20)
        .y_labels(10)
        //.disable_mesh()
        .x_label_formatter(&|v| format!("{:.1}", v))
        .y_label_formatter(&|v| format!("{:.1}", v))
        .draw()
        .unwrap();

    //println!("processing X1 for plot");

    cc.draw_series(LineSeries::new(
        x_axis.values().map(|x| (x, values[x as usize])),
        &RED,
    ))
    .unwrap()
    .label("Sine")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    //println!("processing X2 for plot");

    /*cc.configure_series_labels()
    .border_style(BLACK)
    .draw()
    .unwrap();*/

    root_area.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    //println!("processing X3 for plot");
}

pub fn plot_complex<T: Into<f64> + Clone>(filename: &str, label: &str, values: &[Complex<T>]) {
    let values_re: Vec<f64> = values.iter().cloned().map(|x| x.re.into()).collect();
    let values_im: Vec<f64> = values.iter().cloned().map(|x| x.im.into()).collect();

    let root_area = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();
    root_area.fill(&WHITE).unwrap();
    let x_axis = (0 as f64..values.len() as f64).step(1.0);

    let x_space = 0.0..values.len() as f64;
    let y_min = f64::min(
        values_re.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        values_im.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
    );
    let y_max = f64::max(
        values_re.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
        values_im.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
    );

    let y_span = y_max - y_min;
    let (y_max, y_min) = (y_min + y_span * 1.2, y_max - y_span * 1.2);
    //let y_min = y_max - y_span * 1.2;

    let y_space = y_min..y_max;

    //println!("processing span for plot");

    let mut cc = ChartBuilder::on(&root_area)
        .margin(5)
        .set_all_label_area_size(50)
        .caption(label, ("sans-serif", 40))
        .build_cartesian_2d(x_space, y_space)
        .unwrap();

    //println!("processing X0 for plot");

    cc.configure_mesh()
        .x_labels(20)
        .y_labels(10)
        //.disable_mesh()
        .x_label_formatter(&|v| format!("{:.1}", v))
        .y_label_formatter(&|v| format!("{:.1}", v))
        .draw()
        .unwrap();

    //println!("processing X1 for plot");

    cc.draw_series(LineSeries::new(
        x_axis.values().map(|x| (x, values_re[x as usize])),
        &RED,
    ))
    .unwrap()
    .label("Sine")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    cc.draw_series(LineSeries::new(
        x_axis.values().map(|x| (x, values_im[x as usize])),
        &plotters::style::BLUE,
    ))
    .unwrap()
    .label("Cosine")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    //println!("processing X2 for plot");

    /*cc.configure_series_labels()
    .border_style(BLACK)
    .draw()
    .unwrap();*/

    root_area.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    //println!("processing X3 for plot");
}
