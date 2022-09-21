use io::Result;

pub fn init() -> Result<()> {
    let log: Logger = ::slog::Logger::root(::slog_stdlog::StdLog.fuse(), slog::o!());
    slog_stdlog::init()?;
    
    let mut display: Display<AvCompositor> = Display::new()?;
    let mut event_loop: EventLoop<CalloopData> = EventLoop::try_new()?;
    
    let state      : AvCompositor          = AvCompositor::new(&mut event_loop, &mut display, log.clone());
    let mut data: CalloopData = CalloopData { state, display : display };
    

    crate::winit::init_winit(&mut event_loop, &mut data, log)?;

    let mut args = std::env::args().skip(1); //: impl Iterator<Item = String>
    let flag: Option<String> = args.next();
    let arg: Option<String>  = args.next();

    match (flag.as_deref(), arg) {
        (Some("-c") | Some("--command"), Some(command)) => {
            Command::new(command).spawn().ok();
        },

        _ => {
            Command::new("alacritty").spawn().ok();
        }
    }

    event_loop.run(None, &mut data, move |_| { /* The compositor is running */ })?;
}