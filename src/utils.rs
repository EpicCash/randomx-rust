use num_bigint::BigUint;

pub fn from_u32_to_u256(v: &[u32; 8]) -> BigUint {
    let mut result = BigUint::from(0u32);
    for (i, &value) in v.iter().enumerate() {
        let part = BigUint::from(value) << (i * 32);
        result += part;
    }
    result
}

#[cfg(test)]
mod test {
    use num_bigint::BigUint;
   

    #[test]
    fn test_u32_to_u256() {
        let v = [1; 8];
        let result = crate::utils::from_u32_to_u256(&v);
        let expected: BigUint = (0..8).fold(BigUint::from(0u32), |acc, i| {
            acc + (BigUint::from(1u32) << (i * 32))
        });
        assert_eq!(expected, result);
        assert_eq!([1; 8], v);
    }

    #[test]
    fn test_u32_to_u256_simple() {
        let mut v = [0; 8];
        v[0] = 1;
        let result = crate::utils::from_u32_to_u256(&v);
        let expected = BigUint::from(1u32);
        assert_eq!(expected, result);
    }
}
