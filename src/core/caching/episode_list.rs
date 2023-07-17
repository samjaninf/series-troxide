use std::{collections::HashSet, io::ErrorKind};

use crate::core::{
    api::{
        deserialize_json,
        episodes_information::{get_episode_list, Episode},
        ApiError,
    },
    caching::CACHER,
};
use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use tracing::info;

use super::{read_cache, write_cache, CacheFilePath};

#[derive(Clone, Debug)]
pub struct EpisodeList {
    episodes: Vec<Episode>,
}

impl EpisodeList {
    pub async fn new(series_id: u32) -> Result<Self, ApiError> {
        let episodes_list_path =
            CACHER.get_cache_file_path(CacheFilePath::SeriesEpisodeList(series_id));

        let json_string = match read_cache(&episodes_list_path).await {
            Ok(json_string) => json_string,
            Err(err) => {
                info!("falling back online for 'episode_list' for series id: {series_id}");
                let (episodes, json_string) = get_episode_list(series_id).await?;

                if err.kind() == ErrorKind::NotFound {
                    write_cache(&json_string, &episodes_list_path).await;
                }
                return Ok(Self { episodes });
            }
        };

        let episodes = deserialize_json::<Vec<Episode>>(&json_string)?;
        Ok(Self { episodes })
    }

    pub fn get_episode(&self, season_number: u32, episode_number: u32) -> Option<&Episode> {
        self.episodes.iter().find(|episode| {
            (episode.season == season_number) && (episode.number == Some(episode_number))
        })
    }

    pub fn get_episodes(&self, season: u32) -> Vec<&Episode> {
        self.episodes
            .iter()
            .filter(|episode| episode.season == season)
            .collect()
    }

    // /// Get the total number of all episodes in the Series
    // pub fn get_total_episodes(&self) -> usize {
    //     self.episodes.len()
    // }

    /// Get the total number of all watchable episodes in the Series
    pub fn get_total_watchable_episodes(&self) -> usize {
        self.episodes
            .iter()
            .filter(|episode| Self::is_episode_watchable(episode) == Some(true))
            .count()
    }

    /// Returns the number of all seasons available and their total episodes as a tuple (season_no, total_episodes)
    pub fn get_season_numbers_with_total_episode(&self) -> Vec<(u32, TotalEpisodes)> {
        let seasons: HashSet<u32> = self.episodes.iter().map(|episode| episode.season).collect();
        let mut seasons: Vec<u32> = seasons.iter().copied().collect();
        seasons.sort();

        seasons
            .into_iter()
            .map(|season| {
                let total_episodes = self.get_episodes(season).len();
                let total_watchable_episodes = self
                    .get_episodes(season)
                    .into_iter()
                    .filter(|episode| Self::is_episode_watchable(episode) == Some(true))
                    .count();
                (
                    season,
                    TotalEpisodes::new(total_episodes, total_watchable_episodes),
                )
            })
            .collect()
    }

    /// Returns the number of all seasons available and their total episodes as a tuple (season_no, total_episodes)
    pub fn get_season_numbers_with_total_watchable_episode(&self) -> Vec<(u32, usize)> {
        let seasons: HashSet<u32> = self.episodes.iter().map(|episode| episode.season).collect();
        let mut seasons: Vec<u32> = seasons.iter().copied().collect();
        seasons.sort();

        seasons
            .into_iter()
            .map(|season| {
                let total_episodes = self
                    .get_episodes(season)
                    .into_iter()
                    .filter(|episode| Self::is_episode_watchable(episode) == Some(true))
                    .count();
                (season, total_episodes)
            })
            .collect()
    }

    /// Tells if the episode is watchable or not based on the current time and the episode release time
    ///
    /// This method returns an optional bool as an episode my not have airstamp associated with it hence
    /// the method can not infer that information.
    pub fn is_episode_watchable(episode: &Episode) -> Option<bool> {
        let airstamp = DateTime::parse_from_rfc3339(episode.airstamp.as_ref()?)
            .unwrap()
            .with_timezone(&Local);
        let local_time = Utc::now().with_timezone(&Local);
        Some(airstamp <= local_time)
    }

    /// Returns the previous episode from the current time
    ///
    /// This method is also useful when finding the maximum watchable episode
    /// as you can not watch an episode that is released in the future.
    pub fn get_previous_episode(&self) -> Option<&Episode> {
        let mut episodes_iter = self.episodes.iter().peekable();
        while let Some(episode) = episodes_iter.next() {
            if let Some(peeked_episode) = episodes_iter.peek() {
                if !Self::is_episode_watchable(peeked_episode)? {
                    return Some(episode);
                }
            } else {
                return Some(episode);
            }
        }
        None
    }

    /// Returns the next episode from the current time
    pub fn get_next_episode(&self) -> Option<&Episode> {
        self.episodes
            .iter()
            .find(|episode| Self::is_episode_watchable(episode) == Some(false))
    }

    /// Returns the next episode and it's release time
    pub fn get_next_episode_and_time(&self) -> Option<(&Episode, EpisodeReleaseTime)> {
        let next_episode = self.get_next_episode()?;
        let next_episode_airstamp = next_episode.airstamp.as_ref()?;

        let release_time = EpisodeReleaseTime::from_rfc3339_str(next_episode_airstamp);
        Some((next_episode, release_time))
    }
}

#[derive(Clone, Debug)]
pub struct TotalEpisodes {
    all_episodes: usize,
    all_watchable_episodes: usize,
}

impl TotalEpisodes {
    fn new(all_episodes: usize, all_watchable_episodes: usize) -> Self {
        Self {
            all_episodes,
            all_watchable_episodes,
        }
    }

    /// Retrieves all the episodes
    pub fn get_all_episodes(&self) -> usize {
        self.all_episodes
    }

    /// Retrieves all the watchable episodes
    pub fn get_all_watchable_episodes(&self) -> usize {
        self.all_watchable_episodes
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct EpisodeReleaseTime {
    release_time: DateTime<Local>,
}

impl EpisodeReleaseTime {
    pub fn new(release_time: DateTime<Utc>) -> Self {
        Self {
            release_time: release_time.with_timezone(&Local),
        }
    }

    fn from_rfc3339_str(str: &str) -> Self {
        Self {
            release_time: DateTime::parse_from_rfc3339(str)
                .unwrap()
                .with_timezone(&Local),
        }
    }

    /// Returns the remaining time for an episode to be released
    pub fn get_remaining_release_time(&self) -> Option<String> {
        let local_time = Utc::now().with_timezone(&Local);

        if self.release_time > local_time {
            let time_diff = self.release_time - local_time;

            if time_diff.num_weeks() != 0 {
                return Some(format!("{} weeks", time_diff.num_weeks()));
            }
            if time_diff.num_days() != 0 {
                return Some(format!("{} days", time_diff.num_days()));
            }
            if time_diff.num_hours() != 0 {
                return Some(format!("{} hours", time_diff.num_hours()));
            }
            if time_diff.num_minutes() != 0 {
                return Some(format!("{} minutes", time_diff.num_minutes()));
            }
            Some(String::from("Now"))
        } else {
            None
        }
    }

    /// Returns the remaining full date and time for an episode to be released
    pub fn get_full_release_date_and_time(&self) -> String {
        /// appends zero the minute digit if it's below 10 for better display
        fn append_zero(num: u32) -> String {
            if num < 10 {
                format!("0{num}")
            } else {
                format!("{num}")
            }
        }

        let (is_pm, hour) = self.release_time.hour12();
        let pm_am = if is_pm { "p.m." } else { "a.m." };

        let minute = append_zero(self.release_time.minute());

        format!(
            "{} {} {}:{} {}",
            self.release_time.date_naive(),
            self.release_time.weekday(),
            hour,
            minute,
            pm_am
        )
    }
}

/// Returns the remaining time for an episode to be released
pub fn get_release_remaining_time(episode: &Episode) -> Option<String> {
    let airstamp = DateTime::parse_from_rfc3339(episode.airstamp.as_ref()?)
        .unwrap()
        .with_timezone(&Local);
    let local_time = Utc::now().with_timezone(&Local);

    if airstamp > local_time {
        let time_diff = airstamp - local_time;

        if time_diff.num_weeks() != 0 {
            return Some(format!("{} weeks", time_diff.num_weeks()));
        }
        if time_diff.num_days() != 0 {
            return Some(format!("{} days", time_diff.num_days()));
        }
        if time_diff.num_hours() != 0 {
            return Some(format!("{} hours", time_diff.num_hours()));
        }
        if time_diff.num_minutes() != 0 {
            return Some(format!("{} minutes", time_diff.num_minutes()));
        }
        Some(String::from("Now"))
    } else {
        None
    }
}