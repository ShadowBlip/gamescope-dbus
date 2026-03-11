#[macro_export]
macro_rules! property_change_dispatch {
    ($event:expr,$iface:expr,$iface_ref:expr,
        {$($atom:path => $method:ident),*$(,)?}, // $iface instance methods
        {$($primary_atom:path => $primary_method:path),*$(,)?} // primary interface methods
    ) => {
        $(if $event == $atom.to_string() {
            $iface
            .$method($iface_ref.signal_context())
            .await
            .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        } else )+
        $(if $event == $primary_atom.to_string() {
            $primary_method($iface_ref.signal_context())
            .await
            .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        } else )+ {}
    };
}
