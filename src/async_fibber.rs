/// given index 'n' and a cache, recursively generate fibonacci numbers, caching the results
pub fn fib(n: usize, cache: &mut Vec<u64>) -> Result<u64, String> {
    // using the magic of match to handle the initial conditions,
    // and then recursion to handle the rest.
    if let Some(result) = cache.get(n) {
        return Ok(*result);
    } else {
        // cache miss, recurses back until the first cache hit,
        // then populates and returns from the cache up the stack
        let minus2 = fib(n - 2, cache)?;
        let minus1 = fib(n - 1, cache)?;
        let (result, overflow) = minus2.overflowing_add(minus1);
        if overflow {
            Err(format!("Overflow at fibonacci index {}", n))
        } else {
            &cache.push(result);
            fib(n, cache)
        }
    }
}

/// generate correctly-seeded fibonnaci cache vec
pub fn fib_cache() -> Vec<u64> {
    // cache is always instantiated with first two fib seeds
    vec![0, 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// ensure sane value and cache at n: 0
    fn fib0() {
        let mut cache = fib_cache();
        assert_eq!(fib(0, &mut cache), Ok(0));
        // fib_cache is always primed with two values
        assert_eq!(cache.len(), 2);
    }
    #[test]
    /// ensure sane value and cache at n: 0
    fn fib1() {
        let mut cache = fib_cache();
        assert_eq!(fib(1, &mut cache), Ok(1));
        assert_eq!(cache.len(), 2);
    }
    #[test]
    /// ensure sane value and cache at n: 10, cache should be len n + 1
    fn fib10() {
        let mut cache = fib_cache();
        assert_eq!(fib(10, &mut cache), Ok(55));
        assert_eq!(cache.len(), 11);
    }
    #[test]
    /// ensure sane value and cache at n: 93, max n value before overflow
    fn fib93() {
        let mut cache = fib_cache();
        assert_eq!(fib(93, &mut cache), Ok(12200160415121876738));
        assert_eq!(cache.len(), 94);
    }
    #[test]
    /// value should overflow, cache should not grow beyond 94
    fn fib94() {
        let mut cache = fib_cache();
        assert!(fib(94, &mut cache).is_err());
        assert_eq!(cache.len(), 94);
    }
    #[test]
    /// error should still be returned, cache should still be overflow size
    fn fib200() {
        let mut cache = fib_cache();
        assert!(fib(200, &mut cache).is_err());
        assert_eq!(cache.len(), 94);
    }
}
