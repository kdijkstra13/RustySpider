use std::error::Error;
use log::info;
use crate::modules::types::Content;

pub trait Searchable {
    fn to_query(&self) -> Result<String, Box<dyn Error>>;
    fn to_negative(&self) -> Result<String, Box<dyn Error>>;
}

pub trait Predictable {
    fn predict_new_content(&self) -> Result<Vec<Content>, Box<dyn Error>>;
}

impl Content {
    pub fn new(title:  impl Into<String>,
               negative: impl Into<String>,
               first_prefix: impl Into<String>,
               first: u32,
               second_prefix: impl Into<String>,
               second: u32,
               digits: usize,
               postfix:  impl Into<String>) -> Self {
        Self {
            title: title.into(),
            negative: negative.into(),
            first_prefix: first_prefix.into(),
            first,
            second_prefix: second_prefix.into(),
            second,
            digits,
            postfix: postfix.into(),
        }
    }
}

impl Predictable for Content {
    fn predict_new_content(&self) -> Result<Vec<Content>, Box<dyn Error>> {
        let mut next_episode = self.clone();
        let mut next_season = self.clone();
        next_episode.second +=1;
        next_season.second =1;
        next_season.first +=1;
        let result = vec![next_episode, next_season];
        Ok(result)
    }
}

impl Searchable for Content {
    fn to_query(&self) -> Result<String, Box<dyn Error>> {
        let posf = if self.postfix.is_empty() {
            String::new()
        } else {
            format!(" {}", self.postfix)
        };
        let result = format!("{} {}{:0digits$}{}{:0digits$}{}",
                            self.title,
                            self.first_prefix, self.first,
                            self.second_prefix, self.second,
                            posf,
                            digits=self.digits);
        info!("Content has created query: {}", &result);
        Ok(result)
    }

    fn to_negative(&self) -> Result<String, Box<dyn Error>> {
        let result = self.negative.clone();
        info!("Content has created negative: {}", &result);
        Ok(result)
    }

}
