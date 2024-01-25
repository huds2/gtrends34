use std::thread;
use std::{collections::HashMap, path::Path};
use std::time::{SystemTime, Duration};
use chrono::NaiveDate;
use csv::ReaderBuilder;
use thirtyfour::prelude::*;
use undetected_chromedriver::chrome_caps;

pub struct GTrends {
    driver: WebDriver,
    download_path: String,
    timeout_ms: u128,
    retry_ms: u64
}

#[derive(Debug)]
pub struct Report {
    pub keyword: String,
    pub timestamps: Vec<(NaiveDate, i32)>,
}

impl GTrends {
    /// Creates GTrends struct. ```path``` is a directory, where the temprary files will be
    /// downloaded to. If the directory contains a file named "multiTimeline.csv", it will be
    /// deleted.
    pub async fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut caps = DesiredCapabilities::chrome();
        let mut prefs = HashMap::new();
        prefs.insert("download.default_directory", path);
        caps.add_chrome_option("prefs", prefs)?;
        let driver = chrome_caps(caps).await?;
        // Doing this to get cookies
        // If you don't do this you will be getting error 429
        driver.goto("https://trends.google.com").await?;

        Ok(GTrends {
            driver,
            download_path: path.to_string(),
            timeout_ms: 10000,
            retry_ms: 100
        })
    }

    async fn wait_for_download(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(filename);
        let now = SystemTime::now();

        while now.elapsed()?.as_millis() < self.timeout_ms && !path.exists() {
            thread::sleep(Duration::from_millis(self.retry_ms));
        }
        Ok(())
    }

    fn clear_dir(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(filename);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
    
    /// Get statistics about a single keyword
    pub async fn get_keyword(&self, keyword: &str) -> Result<Report, Box<dyn std::error::Error>> {
        let url = format!("https://trends.google.com/trends/explore?date=all&geo=US&hl=en-US&q={}", keyword);
        let path = self.download_path.clone() + "/multiTimeline.csv";
        let driver = &self.driver;

        self.clear_dir(&path)?;

        driver.goto(url).await?;
        let download_button_xpath = "/html/body/div[3]/div[2]/div/md-content/div/div/div[1]/trends-widget/ng-include/widget/div/div/div/widget-actions/div/button[1]";
        let elem = driver.query(By::XPath(download_button_xpath)).first().await?;
        elem.wait_until().displayed().await?;
        let download_button = driver.find(By::XPath(download_button_xpath)).await?;
        download_button.click().await?;

        self.wait_for_download(&path).await?;

        let mut rdr = ReaderBuilder::new()
            .flexible(true)
            .has_headers(true)
            .from_path(path)?;
        let mut timestamps: Vec<(NaiveDate, i32)> = vec![];

        for result in rdr.records().skip(1) {
            let row = result?;
            let date_str = row[0].to_string() + "-01";
            let level_str: &str = &row[1];
            // Hacky way to do so, because chrono does not want to construct a date without a day
            let time = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap();
            let level: i32 = level_str.parse()?;
            timestamps.push((time, level));
        }

        Ok(Report {
            keyword: keyword.to_string(),
            timestamps
        })
    }
}
