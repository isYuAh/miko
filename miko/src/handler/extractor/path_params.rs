#[derive(Debug, Clone)]
pub struct PathParams(pub Vec<(String, String)>);

impl<'a> From<&matchit::Params<'a, 'a>> for PathParams {
  fn from(p: &matchit::Params<'a, 'a>) -> Self {
    Self(p.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
  }
}

impl PathParams {
  #[inline] pub fn by_index(&self, index: usize) -> Option<String> {
    self.0.get(index).map(|(_, v)| v.clone())
  }

  pub fn shift_count(&self, count: usize) -> PathParams {
    if count >= self.0.len() {
      PathParams(Vec::new())
    } else {
      PathParams(self.0[count..].to_vec())
    }
  }
}