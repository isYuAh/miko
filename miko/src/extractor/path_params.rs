#[derive(Debug, Clone)]
/// 路径参数集合，按声明顺序存储 (name, value)
pub struct PathParams(pub Vec<(String, String)>);

impl<'a> From<&matchit::Params<'a, 'a>> for PathParams {
    fn from(p: &matchit::Params<'a, 'a>) -> Self {
        Self(
            p.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }
}

impl PathParams {
    /// 按序号获取某个参数的值
    #[inline]
    pub fn by_index(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|(_, v)| v.clone())
    }

    /// 丢弃前 count 个参数，常用于 nest 的前缀偏移
    pub fn shift_count(&self, count: usize) -> PathParams {
        if count >= self.0.len() {
            PathParams(Vec::new())
        } else {
            PathParams(self.0[count..].to_vec())
        }
    }
}
