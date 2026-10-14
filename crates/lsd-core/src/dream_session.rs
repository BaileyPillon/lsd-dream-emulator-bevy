/// Services called by the original session policy. Scene/day implementations
/// own their effects and may change session state during a callback.
pub trait DreamSessionHost {
    type Error;
    fn base_tick(&mut self, session: &mut DreamSession, code: i32) -> Result<(), Self::Error>;
    fn base_notify(&mut self, session: &mut DreamSession, code: i32) -> Result<(), Self::Error>;
    fn sender_type(&self) -> u32;
    fn start_day(&mut self) -> Result<i32, Self::Error>;
    fn end_day(&mut self, mode: i32) -> Result<i32, Self::Error>;
    fn cinematic(&mut self) -> Result<[i16; 4], Self::Error>;
    fn current_map(&mut self) -> Result<i32, Self::Error>;
    fn stop_map(&mut self, source: Option<u32>) -> Result<(), Self::Error>;
    fn destroy_map(&mut self) -> Result<(), Self::Error>;
    fn create_map(&mut self, stage: i32) -> Result<(), Self::Error>;
    fn attach_map(&mut self) -> Result<(), Self::Error>;
    fn run_map(&mut self) -> Result<(), Self::Error>;
    fn finish(&mut self, code: i32) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DreamSession {
    pub state: i32,
    pub result: i32,
}
impl DreamSession {
    fn open_map<H: DreamSessionHost>(&mut self, host: &mut H, stage: i32) -> Result<(), H::Error> {
        host.create_map(stage)?;
        host.attach_map()?;
        host.run_map()?;
        self.state = 2;
        Ok(())
    }

    pub fn tick<H: DreamSessionHost>(&mut self, host: &mut H, code: i32) -> Result<(), H::Error> {
        host.base_tick(self, code)?;
        if code != 2 {
            return Ok(());
        }
        match self.state {
            1 => {
                let stage = host.start_day()?;
                if stage >= 0 {
                    self.open_map(host, stage)?;
                } else {
                    host.end_day(0)?;
                    self.result = 2;
                    host.finish(3)?;
                }
            }
            3 => {
                host.stop_map(None)?;
                host.destroy_map()?;
                let stage = host.current_map()?;
                self.open_map(host, stage)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn link_code<H: DreamSessionHost>(
        &mut self,
        host: &mut H,
        source: u32,
        code: i32,
    ) -> Result<(), H::Error> {
        match code {
            4 => {
                host.stop_map(Some(source))?;
                host.destroy_map()?;
                self.result = if host.end_day(0)? == 0 {
                    if host.cinematic()?[1] < 0 { 1 } else { 2 }
                } else {
                    3
                };
                host.finish(3)?;
            }
            5 | 6 | 7 | 8 | 10 => self.state = 3,
            12 | 13 => {
                host.stop_map(Some(source))?;
                host.destroy_map()?;
                host.end_day(if code == 12 { 1 } else { 2 })?;
                self.result = 3;
                host.finish(3)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn notify<H: DreamSessionHost>(
        &mut self,
        host: &mut H,
        source: u32,
        code: i32,
    ) -> Result<(), H::Error> {
        host.base_notify(self, code)?;
        let kind = host.sender_type();
        if kind & 0xffff != 0x1f34 && kind & 0xfffff == 0x2f230 {
            self.link_code(host, source, code)?;
        }
        Ok(())
    }
}
