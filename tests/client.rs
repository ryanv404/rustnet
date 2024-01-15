#[cfg(test)]
#[macro_use]
mod common;

#[cfg(test)]
mod get {
    use super::*;
    run_test!(CLIENT: GET deny);
    run_test!(CLIENT: GET html);
    run_test!(CLIENT: GET json);
    run_test!(CLIENT: GET xml);
    run_test!(CLIENT: GET image_jpeg);
    run_test!(CLIENT: GET status_101);
    run_test!(CLIENT: GET status_200);
    run_test!(CLIENT: GET status_301);
    run_test!(CLIENT: GET status_404);
    run_test!(CLIENT: GET status_502);
}

#[cfg(test)]
mod post {
    use super::*;
    run_test!(CLIENT: POST status_201);
    run_test!(CLIENT: POST status_301);
    run_test!(CLIENT: POST status_404);
}

#[cfg(test)]
mod patch {
    use super::*;
    run_test!(CLIENT: PATCH status_200);
    run_test!(CLIENT: PATCH status_301);
    run_test!(CLIENT: PATCH status_404);
}

#[cfg(test)]
mod put {
    use super::*;
    run_test!(CLIENT: PUT status_200);
    run_test!(CLIENT: PUT status_301);
    run_test!(CLIENT: PUT status_404);
}

#[cfg(test)]
mod delete {
    use super::*;
    run_test!(CLIENT: DELETE status_200);
    run_test!(CLIENT: DELETE status_301);
    run_test!(CLIENT: DELETE status_404);
}
