use std::convert::{TryInto, TryFrom};
use uuid::Uuid;
use std::time::{Duration, SystemTime};
use std::str::FromStr;
use std::env;
use std::fmt::Debug;
use rand::RngCore;

pub fn uuidify<P, R>(pid: P, run: R) -> Uuid
    where
        P: TryInto<u64>,
        R: TryInto<u64>,
        <P as TryInto<u64>>::Error: Debug,
        <R as TryInto<u64>>::Error: Debug,
{
    try_uuidify(pid, run).unwrap()
}

#[derive(Debug)]
pub enum Bimorphic<U, V> {
    A(U),
    B(V),
}

pub fn try_uuidify<P, R>(
    pid: P,
    run: R,
) -> Result<Uuid, Bimorphic<<P as TryInto<u64>>::Error, <R as TryInto<u64>>::Error>>
    where
        P: TryInto<u64>,
        R: TryInto<u64>,
{
    let pid = pid.try_into().map_err(|err| Bimorphic::A(err))? as u128;
    let run = run.try_into().map_err(|err| Bimorphic::B(err))? as u128;
    Ok(Uuid::from_u128(pid << 64 | run))
}

pub fn deuuid<P, R>(uuid: Uuid) -> (P, R)
    where
        P: TryFrom<u64>,
        <P as TryFrom<u64>>::Error: Debug,
        R: TryFrom<u64>,
        <R as TryFrom<u64>>::Error: Debug,
{
    try_deuuid(uuid).unwrap()
}

pub fn try_deuuid<P, R>(
    uuid: Uuid,
) -> Result<(P, R), Bimorphic<<P as TryFrom<u64>>::Error, <R as TryFrom<u64>>::Error>>
    where
        P: TryFrom<u64>,
        R: TryFrom<u64>,
{
    let val = uuid.as_u128();
    let pid = <P>::try_from((val >> 64) as u64).map_err(|err| Bimorphic::A(err))?;
    let run = <R>::try_from(val as u64).map_err(|err| Bimorphic::B(err))?;
    Ok((pid, run))
}

pub fn timed<F, R>(f: F) -> (R, Duration)
    where
        F: Fn() -> R,
{
    let start = SystemTime::now();
    (
        f(),
        SystemTime::now()
            .duration_since(start)
            .unwrap_or(Duration::new(0, 0)),
    )
}

pub fn scale() -> usize {
    get_env::<usize, _>("SCALE", || 1)
}

pub fn seed() -> u64 {
    get_env("SEED", || rand::thread_rng().next_u64())
}

pub fn get_env<T, D>(key: &str, def: D) -> T
    where
        T: FromStr,
        T::Err: std::fmt::Debug,
        D: Fn() -> T,
{
    match env::var(key) {
        Ok(str) => T::from_str(&str).expect(&format!("invalid {} value '{}'", key, str)),
        Err(_) => def(),
    }
}

#[cfg(test)]
mod tests;