use serde::Deserialize;

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    /// Build database URL from configuration
    pub fn url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "cqrs_events".to_string(),
            max_connections: 10,
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            log_level: "info".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_url() {
        let config = DatabaseConfig::default();
        let url = config.url();
        assert_eq!(url, "postgres://postgres:postgres@localhost:5432/cqrs_events");
    }
}
