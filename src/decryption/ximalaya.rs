pub const XMLY_SCRAMBLE_SIZE: usize = 1024;
pub const X2M_CONTENT_KEY_SIZE: usize = 0x04;
pub const X3M_CONTENT_KEY_SIZE: usize = 0x20;

pub type X2MContentKey = [u8; X2M_CONTENT_KEY_SIZE];
pub type X3MContentKey = [u8; X3M_CONTENT_KEY_SIZE];
pub type ScrambleTable = [u16; XMLY_SCRAMBLE_SIZE];

mod detail {
    use super::{ScrambleTable, X2MContentKey, X3MContentKey, XMLY_SCRAMBLE_SIZE};
    use crate::{
        decryptor::{BaseDecryptorData, DecryptError, Decryptor},
        utils::array_ext::ArrayExtension,
    };

    enum State {
        DecryptHeader,
        PassThrough,
    }

    pub struct Ximalaya<T> {
        data: BaseDecryptorData,
        state: State,
        key: T,
        scramble_table: ScrambleTable,
    }

    impl<const KEY_SIZE: usize> Ximalaya<[u8; KEY_SIZE]> {
        pub fn new(key: [u8; KEY_SIZE], scramble_table: ScrambleTable) -> Self {
            let data = BaseDecryptorData::new();
            Ximalaya {
                data,
                key,
                state: State::DecryptHeader,
                scramble_table,
            }
        }

        fn do_header_decryption(&mut self) {
            let mut output = vec![0u8; XMLY_SCRAMBLE_SIZE];
            for (i, v) in output.iter_mut().enumerate() {
                let idx = usize::from(self.scramble_table[i]);
                *v = self.data.buf_in[idx] ^ self.key.get_mod_n(i);
            }
            self.data.buf_out.append(&mut output);
            self.data.consume_bytes(XMLY_SCRAMBLE_SIZE);
        }
    }

    impl<const KEY_SIZE: usize> Decryptor for Ximalaya<[u8; KEY_SIZE]> {
        fn get_data(&self) -> &BaseDecryptorData {
            &self.data
        }
        fn get_data_mut(&mut self) -> &mut BaseDecryptorData {
            &mut self.data
        }

        fn write(&mut self, data: &[u8]) -> Result<(), DecryptError> {
            let mut p = data;

            while !p.is_empty() {
                match self.state {
                    State::DecryptHeader => {
                        if self.data.read_until_offset(&mut p, XMLY_SCRAMBLE_SIZE) {
                            self.do_header_decryption();
                            self.state = State::PassThrough;
                        }
                    }
                    State::PassThrough => {
                        self.data.buf_out.extend_from_slice(p);
                        self.data.offset += p.len();
                        break;
                    }
                }
            }

            Ok(())
        }
    }

    pub fn new_x2m(key: X2MContentKey, scramble_table: ScrambleTable) -> impl Decryptor {
        Ximalaya::new(key, scramble_table)
    }

    pub fn new_x3m(key: X3MContentKey, scramble_table: ScrambleTable) -> impl Decryptor {
        Ximalaya::new(key, scramble_table)
    }
}

pub use detail::new_x2m;
pub use detail::new_x3m;

#[cfg(test)]
pub mod test {
    use super::{ScrambleTable, XMLY_SCRAMBLE_SIZE};
    use crate::{
        decryption::ximalaya::{X2M_CONTENT_KEY_SIZE, X3M_CONTENT_KEY_SIZE},
        utils::array_ext::ByteSliceExt,
        utils::test_util::test::{decrypt_test_content, generate_test_data, TEST_SIZE_1MB},
    };

    #[test]
    fn test_x2m() {
        let test_data = generate_test_data(TEST_SIZE_1MB, "x2m-test-data");
        let x2m_content_key = generate_test_data(X2M_CONTENT_KEY_SIZE, "x2m content key");

        let mut x2m_scramble_table: ScrambleTable = [0u16; XMLY_SCRAMBLE_SIZE];
        for (i, v) in x2m_scramble_table.iter_mut().enumerate() {
            *v = i as u16;
        }
        let table_size = x2m_scramble_table.len();
        let x2m_scramble_seed = generate_test_data(table_size * 2, "x2m seed");
        for i in 0..table_size {
            let n = x2m_scramble_seed.read_le::<u16>(i * 2) as usize;
            x2m_scramble_table.swap(i, n % table_size);
        }

        assert_eq!(x2m_content_key.len(), X2M_CONTENT_KEY_SIZE);

        let mut decryptor = super::new_x2m(
            x2m_content_key
                .try_into()
                .expect("could not format to array"),
            x2m_scramble_table,
        );
        let result = decrypt_test_content(&mut decryptor, test_data.as_ref());
        assert_eq!(
            result,
            "fd1ac1c4750f48b8d3c9562013f1c3202b12e45137b344995eda32a4f6b8a61f"
        );
    }

    #[test]
    fn test_x3m() {
        let test_data = generate_test_data(TEST_SIZE_1MB, "x3m-test-data");
        let x3m_content_key = generate_test_data(X3M_CONTENT_KEY_SIZE, "x3m content key");

        let mut x3m_scramble_table: ScrambleTable = [0u16; XMLY_SCRAMBLE_SIZE];
        for (i, v) in x3m_scramble_table.iter_mut().enumerate() {
            *v = i as u16;
        }
        let table_size = x3m_scramble_table.len();
        let x3m_scramble_seed = generate_test_data(table_size * 2, "x3m seed");
        for i in 0..table_size {
            let n = x3m_scramble_seed.read_le::<u16>(i * 2) as usize;
            x3m_scramble_table.swap(i, n % table_size);
        }

        assert_eq!(x3m_content_key.len(), X3M_CONTENT_KEY_SIZE);

        let mut decryptor = super::new_x3m(
            x3m_content_key
                .try_into()
                .expect("could not format to array"),
            x3m_scramble_table,
        );
        let result = decrypt_test_content(&mut decryptor, test_data.as_ref());
        assert_eq!(
            result,
            "a10bbfdcdbd388373361da6baf35c80b725f7310c3eca29d7dcf228e397a8c5a"
        );
    }
}
