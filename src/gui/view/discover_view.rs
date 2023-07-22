use std::sync::mpsc;

use super::series_view;
use crate::core::api::episodes_information::Episode;
use crate::core::api::series_information::SeriesMainInformation;
use crate::core::api::tv_schedule::{get_episodes_with_country, get_episodes_with_date};
use crate::core::api::updates::show_updates::*;
use crate::core::settings_config::locale_settings;
use crate::gui::assets::icons::BINOCULARS_FILL;
use crate::gui::troxide_widget::series_poster::{Message as SeriesPosterMessage, SeriesPoster};
use crate::gui::{troxide_widget, Message as GuiMessage, Tab};
use searching::Message as SearchMessage;

use iced::widget::{column, container, scrollable, text, vertical_space};
use iced::{Command, Element, Length, Renderer};

use iced_aw::floating_element;
use iced_aw::wrap::Wrap;
use iced_aw::Spinner;

#[derive(Default, PartialEq)]
enum LoadState {
    #[default]
    Loading,
    Loaded,
}

#[derive(Clone, Debug)]
pub enum Message {
    //LoadSchedule, TODO: implement a refresh button in the discover view
    ScheduleLoaded(Vec<Episode>),
    CountryScheduleLoaded(Vec<Episode>),
    SeriesUpdatesLoaded(Vec<SeriesMainInformation>),
    EpisodePosterAction(/*episode poster index*/ usize, SeriesPosterMessage),
    CountryEpisodePosterAction(/*episode poster index*/ usize, SeriesPosterMessage),
    SeriesPosterAction(/*series poster index*/ usize, SeriesPosterMessage),
    SearchAction(SearchMessage),
    SeriesSelected(Box<SeriesMainInformation>),
    ShowOverlay,
    HideOverlay,
}

pub struct DiscoverTab {
    load_state: LoadState,
    show_overlay: bool,
    search_state: searching::Search,
    new_episodes: Vec<SeriesPoster>,
    new_country_episodes: Vec<SeriesPoster>,
    series_updates: Vec<SeriesPoster>,
    series_page_sender: mpsc::Sender<(series_view::Series, Command<series_view::Message>)>,
}

impl DiscoverTab {
    pub fn new(
        series_page_sender: mpsc::Sender<(series_view::Series, Command<series_view::Message>)>,
    ) -> (Self, Command<Message>) {
        (
            Self {
                load_state: LoadState::Loading,
                show_overlay: false,
                search_state: searching::Search::default(),
                new_episodes: vec![],
                new_country_episodes: vec![],
                series_updates: vec![],
                series_page_sender,
            },
            load_discover_schedule_command(),
        )
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // TODO: implement a refresh button in the discover view
            // Message::LoadSchedule => {
            //     self.load_state = LoadState::Loading;
            //     load_discover_schedule_command()
            // }
            Message::ScheduleLoaded(episodes) => {
                self.load_state = LoadState::Loaded;

                let mut episode_posters = Vec::with_capacity(episodes.len());
                let mut commands = Vec::with_capacity(episodes.len());
                for (index, episode) in episodes.into_iter().enumerate() {
                    let (poster, command) = SeriesPoster::from_episode_info(index, episode);
                    episode_posters.push(poster);
                    commands.push(command);
                }

                self.new_episodes = episode_posters;
                Command::batch(commands).map(|message| {
                    Message::EpisodePosterAction(message.get_id().unwrap_or(0), message)
                })
            }
            Message::EpisodePosterAction(index, message) => {
                if let SeriesPosterMessage::SeriesPosterPressed(series_information) = message {
                    self.show_overlay = false;
                    return Command::perform(async {}, |_| {
                        Message::SeriesSelected(series_information)
                    });
                }
                self.new_episodes[index]
                    .update(message)
                    .map(move |message| Message::EpisodePosterAction(index, message))
            }
            Message::SeriesUpdatesLoaded(series) => {
                let mut series_infos = Vec::with_capacity(series.len());
                let mut series_poster_commands = Vec::with_capacity(series.len());
                for (index, series_info) in series.into_iter().enumerate() {
                    let (series_poster, series_poster_command) =
                        SeriesPoster::new(index, series_info);
                    series_infos.push(series_poster);
                    series_poster_commands.push(series_poster_command);
                }
                self.series_updates = series_infos;

                Command::batch(series_poster_commands).map(|message| {
                    Message::SeriesPosterAction(message.get_id().unwrap_or(0), message)
                })
            }
            Message::SeriesPosterAction(index, message) => {
                if let SeriesPosterMessage::SeriesPosterPressed(series_information) = message {
                    self.show_overlay = false;
                    return Command::perform(async {}, |_| {
                        Message::SeriesSelected(series_information)
                    });
                }
                self.series_updates[index]
                    .update(message)
                    .map(move |message| Message::SeriesPosterAction(index, message))
            }
            Message::SearchAction(message) => {
                if let SearchMessage::SeriesResultPressed(series_id) = message {
                    self.series_page_sender
                        .send(series_view::Series::from_series_id(series_id))
                        .expect("failed to send series page");
                    self.show_overlay = false;
                    return Command::none();
                };
                self.search_state.update(message)
            }
            Message::ShowOverlay => {
                self.show_overlay = true;
                Command::none()
            }
            Message::HideOverlay => {
                self.show_overlay = false;
                Command::none()
            }
            Message::SeriesSelected(series_info) => {
                self.series_page_sender
                    .send(series_view::Series::from_series_information(*series_info))
                    .expect("failed to send series page");
                Command::none()
            }
            Message::CountryScheduleLoaded(episodes) => {
                self.load_state = LoadState::Loaded;

                let mut episode_posters = Vec::with_capacity(episodes.len());
                let mut commands = Vec::with_capacity(episodes.len());
                for (index, episode) in episodes.into_iter().enumerate() {
                    let (poster, command) = SeriesPoster::from_episode_info(index, episode);
                    episode_posters.push(poster);
                    commands.push(command);
                }
                self.new_country_episodes = episode_posters;
                Command::batch(commands).map(|message| {
                    Message::CountryEpisodePosterAction(message.get_id().unwrap_or(0), message)
                })
            }
            Message::CountryEpisodePosterAction(index, message) => {
                if let SeriesPosterMessage::SeriesPosterPressed(series_information) = message {
                    self.show_overlay = false;
                    return Command::perform(async {}, |_| {
                        Message::SeriesSelected(series_information)
                    });
                }
                self.new_country_episodes[index]
                    .update(message)
                    .map(move |message| Message::CountryEpisodePosterAction(index, message))
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let country = locale_settings::get_country_name_from_settings();
        let underlay: Element<'_, Message, Renderer> = match self.load_state {
            LoadState::Loading => container(Spinner::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into(),
            LoadState::Loaded => column!(scrollable(
                column!(
                    series_posters_loader("Shows Airing Today Globally", &self.new_episodes),
                    series_posters_loader(
                        &format!("Shows Airing Today in {}", country),
                        &self.new_country_episodes
                    ),
                    series_posters_loader("Shows Updates", &self.series_updates),
                )
                .spacing(20)
            )
            .width(Length::Fill))
            .into(),
        };

        let content = floating_element::FloatingElement::new(underlay, || {
            self.search_state.view().1.map(Message::SearchAction)
        })
        .anchor(floating_element::Anchor::North)
        .hide(!self.show_overlay);

        column![
            self.search_state.view().0.map(Message::SearchAction),
            content
        ]
        .spacing(2)
        .padding(10)
        .into()
    }
}

impl Tab for DiscoverTab {
    type Message = GuiMessage;

    fn title(&self) -> String {
        "Discover".to_owned()
    }

    fn tab_label(&self) -> troxide_widget::tabs::TabLabel {
        troxide_widget::tabs::TabLabel::new(self.title(), BINOCULARS_FILL)
    }

    fn content(&self) -> Element<'_, Self::Message> {
        self.view().map(GuiMessage::Discover)
    }
}

/// loads the shows updates and the scheduled episodes of the discover view
fn load_discover_schedule_command() -> Command<Message> {
    let series_updates_command =
        Command::perform(get_show_updates(UpdateTimestamp::Day, Some(5)), |series| {
            Message::SeriesUpdatesLoaded(series.expect("failed to load series updates"))
        });

    let new_episodes_command = Command::perform(get_episodes_with_date(None), |episodes| {
        Message::ScheduleLoaded(episodes.expect("failed to load episodes schedule"))
    });

    let new_country_episodes_command = Command::perform(
        async {
            let country_code = locale_settings::get_country_code_from_settings();
            get_episodes_with_country(&country_code).await
        },
        |episodes| {
            Message::CountryScheduleLoaded(episodes.expect("failed to load episodes schedule"))
        },
    );

    Command::batch([
        series_updates_command,
        new_episodes_command,
        new_country_episodes_command,
    ])
}

/// wraps the given series posters and places a title above them
fn series_posters_loader<'a>(
    title: &str,
    posters: &'a [SeriesPoster],
) -> Element<'a, Message, Renderer> {
    let title = text(title).size(25);

    let wrapped_posters = Wrap::with_elements(
        posters
            .iter()
            .map(|poster| {
                poster.view().map(|message| {
                    Message::SeriesPosterAction(message.get_id().unwrap_or(0), message)
                })
            })
            .collect(),
    )
    .spacing(5.0)
    .line_spacing(5.0)
    .padding(5.0);

    column!(title, vertical_space(10), wrapped_posters)
        .width(Length::Fill)
        .padding(10)
        .into()
}

mod searching {
    use bytes::Bytes;
    use iced::widget::{
        column, container, horizontal_space, image, mouse_area, row, scrollable, text, text_input,
        vertical_space, Column,
    };
    use iced::{Command, Element, Length, Renderer};
    use iced_aw::Spinner;
    use tokio::task::JoinHandle;

    use super::Message as DiscoverMessage;
    use crate::core::api::series_searching;
    use crate::core::caching;
    use crate::gui::styles;

    #[derive(Default)]
    pub enum LoadState {
        Loaded,
        Loading,
        #[default]
        NotLoaded,
    }

    #[derive(Clone, Debug)]
    pub enum Message {
        SearchTermChanged(String),
        SearchTermSearched,
        SearchSuccess(Vec<series_searching::SeriesSearchResult>),
        SearchFail,
        ImagesLoaded(Vec<Option<Bytes>>),
        SeriesResultPressed(/*series id*/ u32),
    }

    #[derive(Default)]
    pub struct Search {
        search_term: String,
        series_search_result: Vec<series_searching::SeriesSearchResult>,
        series_search_results_images: Vec<Option<Bytes>>,
        pub load_state: LoadState,
    }

    impl Search {
        pub fn update(&mut self, message: Message) -> Command<DiscoverMessage> {
            match message {
                Message::SearchTermChanged(term) => {
                    self.search_term = term;
                    return if self.search_term.is_empty() {
                        self.load_state = LoadState::NotLoaded;
                        Command::perform(async {}, |_| DiscoverMessage::HideOverlay)
                    } else {
                        Command::none()
                    };
                }
                Message::SearchTermSearched => {
                    self.load_state = LoadState::Loading;

                    let series_result = series_searching::search_series(self.search_term.clone());

                    let search_status_command = Command::perform(series_result, |res| match res {
                        Ok(res) => DiscoverMessage::SearchAction(Message::SearchSuccess(res)),
                        Err(err) => {
                            println!("{:?}", err);
                            DiscoverMessage::SearchAction(Message::SearchFail)
                        }
                    });

                    let show_overlay_command =
                        Command::perform(async {}, |_| DiscoverMessage::ShowOverlay);

                    return Command::batch([search_status_command, show_overlay_command]);
                }
                Message::SearchSuccess(res) => {
                    self.load_state = LoadState::Loaded;
                    self.series_search_results_images.clear();
                    self.series_search_result = res.clone();
                    let image_command =
                        Command::perform(load_series_result_images(res), |images| {
                            DiscoverMessage::SearchAction(Message::ImagesLoaded(images))
                        });
                    let show_overlay_command =
                        Command::perform(async {}, |_| DiscoverMessage::ShowOverlay);

                    return Command::batch([image_command, show_overlay_command]);
                }
                Message::SearchFail => panic!("Series Search Failed"),
                Message::ImagesLoaded(images) => self.series_search_results_images = images,
                Message::SeriesResultPressed(_) => {
                    unreachable!("Search page should not handle series page result")
                }
            }
            Command::none()
        }

        pub fn view(
            &self,
        ) -> (
            Element<'_, Message, Renderer>,
            Element<'_, Message, Renderer>,
        ) {
            let search_bar = column!(
                vertical_space(10),
                text_input("Search Series", &self.search_term)
                    .width(300)
                    .on_input(Message::SearchTermChanged)
                    .on_submit(Message::SearchTermSearched)
            )
            .width(Length::Fill)
            .align_items(iced::Alignment::Center);

            let menu_widgets: Element<'_, Message, Renderer> = match self.load_state {
                LoadState::Loaded => Column::with_children(load(
                    &self.series_search_result,
                    &self.series_search_results_images,
                ))
                .padding(20)
                .spacing(5)
                .into(),
                LoadState::Loading => container(Spinner::new())
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .center_x()
                    .center_y()
                    .into(),
                LoadState::NotLoaded => container("").into(),
            };

            let menu_widgets = container(menu_widgets)
                .style(styles::container_styles::first_class_container_theme());

            (search_bar.into(), scrollable(menu_widgets).into())
        }
    }

    fn load<'a>(
        series_result: &'a [series_searching::SeriesSearchResult],
        series_images: &[Option<Bytes>],
    ) -> Vec<Element<'a, Message, Renderer>> {
        let mut results = Vec::new();

        for (index, series_result) in series_result.iter().enumerate() {
            results.push(series_result_widget(
                series_result,
                if series_images.is_empty() {
                    None
                } else {
                    series_images[index].clone().take()
                },
            ));
        }
        results
    }

    pub fn series_result_widget(
        series_result: &series_searching::SeriesSearchResult,
        image_bytes: Option<Bytes>,
    ) -> iced::Element<'_, Message, Renderer> {
        let mut row = row!();

        if let Some(image_bytes) = image_bytes {
            let image_handle = image::Handle::from_memory(image_bytes);

            let image = image(image_handle).height(60);
            row = row
                .push(horizontal_space(5))
                .push(image)
                .push(horizontal_space(5));
        }

        // Getting the series genres
        let genres = if !series_result.show.genres.is_empty() {
            let mut genres = String::from("Genres: ");

            let mut series_result_iter = series_result.show.genres.iter().peekable();
            while let Some(genre) = series_result_iter.next() {
                genres.push_str(genre);
                if series_result_iter.peek().is_some() {
                    genres.push_str(", ");
                }
            }
            genres
        } else {
            String::new()
        };

        let mut column = column!(
            text(&series_result.show.name).size(20),
            text(genres).size(15),
        );

        if let Some(premier) = &series_result.show.premiered {
            column = column.push(text(format!("Premiered: {}", premier)).size(13));
        }

        mouse_area(row.push(column))
            .on_press(Message::SeriesResultPressed(series_result.show.id))
            .into()
    }

    async fn load_series_result_images(
        series_results: Vec<series_searching::SeriesSearchResult>,
    ) -> Vec<Option<Bytes>> {
        let mut loaded_results = Vec::with_capacity(series_results.len());
        let handles: Vec<JoinHandle<Option<Bytes>>> = series_results
            .into_iter()
            .map(|result| {
                tokio::task::spawn(async {
                    if let Some(url) = result.show.image {
                        caching::load_image(url.medium_image_url).await
                    } else {
                        None
                    }
                })
            })
            .collect();

        for handle in handles {
            let loaded_result = handle
                .await
                .expect("Failed to await all the search images handles");
            loaded_results.push(loaded_result)
        }
        loaded_results
    }
}
