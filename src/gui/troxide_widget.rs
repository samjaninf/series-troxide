pub mod episode_widget {
    use crate::core::{
        api::tv_maze::episodes_information::Episode as EpisodeInfo, caching, database,
    };
    use crate::gui::assets::icons::EYE_FILL;
    use crate::gui::helpers::{self, season_episode_str_gen};
    pub use crate::gui::message::IndexedMessage;
    use crate::gui::styles;
    use bytes::Bytes;
    use iced::font::Weight;
    use iced::widget::{
        button, checkbox, column, container, image, row, svg, text, Row, Space, Text,
    };
    use iced::{Element, Font, Length, Task};

    #[derive(Clone, Debug)]
    pub enum Message {
        ImageLoaded(Option<Bytes>),
        MarkedWatched(PosterType),
        TrackTaskComplete(bool),
    }

    #[derive(Clone, Copy, Debug)]
    pub enum PosterType {
        Watchlist,
        Season,
    }

    #[derive(Clone)]
    pub struct Episode {
        index: usize,
        series_name: String,
        episode_information: EpisodeInfo,
        series_id: u32,
        episode_image: Option<Bytes>,
        set_watched: bool,
    }

    impl Episode {
        pub fn new(
            index: usize,
            series_id: u32,
            series_name: String,
            episode_information: EpisodeInfo,
        ) -> (Self, Task<IndexedMessage<usize, Message>>) {
            let episode_image = episode_information.image.clone();
            let episode = Self {
                index,
                series_name,
                episode_information,
                series_id,
                episode_image: None,
                set_watched: false,
            };

            let command = if let Some(image) = episode_image {
                Task::perform(
                    caching::load_image(image.medium_image_url, caching::ImageResolution::Medium),
                    Message::ImageLoaded,
                )
                .map(move |message| IndexedMessage::new(index, message))
            } else {
                Task::none()
            };

            (episode, command)
        }

        pub fn is_set_watched(&self) -> bool {
            self.set_watched
        }

        pub fn update(
            &mut self,
            message: IndexedMessage<usize, Message>,
        ) -> Task<IndexedMessage<usize, Message>> {
            match message.message() {
                Message::ImageLoaded(image) => {
                    self.episode_image = image;
                    Task::none()
                }
                Message::MarkedWatched(poster_type) => {
                    let season_number = self.episode_information.season;
                    let episode_number = self.episode_information.number.unwrap();
                    let series_id = self.series_id;
                    let series_name = self.series_name.clone();
                    let episode_index = self.index;

                    match poster_type {
                        PosterType::Watchlist => {
                            self.set_watched = true;
                            if let Some(mut series) = database::DB.get_series(series_id) {
                                series.add_episode_unchecked(season_number, episode_number);
                            } else {
                                let mut series = database::Series::new(series_name, series_id);
                                series.add_episode_unchecked(season_number, episode_number)
                            }

                            Task::none()
                        }
                        PosterType::Season => Task::perform(
                            async move {
                                if let Some(mut series) = database::DB.get_series(series_id) {
                                    series.add_episode(season_number, episode_number).await
                                } else {
                                    let mut series = database::Series::new(series_name, series_id);
                                    series.add_episode(season_number, episode_number).await
                                }
                            },
                            Message::TrackTaskComplete,
                        )
                        .map(move |message| IndexedMessage::new(episode_index, message)),
                    }
                }
                Message::TrackTaskComplete(is_newly_added) => {
                    if !is_newly_added {
                        if let Some(mut series) = database::DB.get_series(self.series_id) {
                            series.remove_episode(
                                self.episode_information.season,
                                self.episode_information.number.unwrap(),
                            );
                        }
                    }
                    Task::none()
                }
            }
        }

        pub fn view(&self, poster_type: PosterType) -> Element<'_, IndexedMessage<usize, Message>> {
            let (poster_width, image_width, image_height) = match poster_type {
                PosterType::Watchlist => (800_f32, 124_f32, 70_f32),
                PosterType::Season => (700_f32, 107_f32, 60_f32),
            };

            let mut content = row!().padding(5).spacing(5).width(poster_width);

            if let Some(image_bytes) = self.episode_image.clone() {
                let image_handle = image::Handle::from_bytes(image_bytes);
                let image = image(image_handle).height(image_height);
                content = content.push(image);
            } else {
                content = content.push(
                    helpers::empty_image::empty_image()
                        .width(image_width)
                        .height(image_height),
                );
            };

            let episode_details = column!(
                heading_widget(self.series_id, &self.episode_information, poster_type),
                date_time_widget(&self.episode_information),
                Space::with_height(5),
                summary_widget(&self.episode_information)
            );

            let content = content.push(episode_details);

            let mut content = container(content);

            if let PosterType::Season = poster_type {
                content =
                    content.style(styles::container_styles::second_class_container_rounded_theme);
            }

            let element: Element<'_, Message> = content.into();

            element.map(|message| IndexedMessage::new(self.index, message))
        }
    }

    fn summary_widget(episode_information: &EpisodeInfo) -> Text<'static> {
        if let Some(summary) = &episode_information.summary {
            let summary = html2text::from_read(summary.as_bytes(), 1000).unwrap_or_default();
            text(summary).size(11)
        } else {
            text("")
        }
    }

    fn date_time_widget(episode_information: &EpisodeInfo) -> Element<'_, Message> {
        if let Ok(release_time) = episode_information.release_time() {
            let prefix = match release_time.is_future() {
                true => "Airing on",
                false => "Aired on",
            };
            text(format!("{} {}", prefix, release_time)).into()
        } else {
            Space::new(0, 0).into()
        }
    }

    fn heading_widget(
        series_id: u32,
        episode_information: &EpisodeInfo,
        poster_type: PosterType,
    ) -> Row<'static, Message> {
        let mark_watched_widget: Element<'_, Message> = match poster_type {
            PosterType::Watchlist => {
                let tracked_icon_handle = svg::Handle::from_memory(EYE_FILL);
                let icon = svg(tracked_icon_handle)
                    .width(17)
                    .height(17)
                    .style(styles::svg_styles::colored_svg_theme);
                button(icon)
                    .style(styles::button_styles::transparent_button_theme)
                    .on_press(Message::MarkedWatched(poster_type))
                    .into()
            }
            PosterType::Season => {
                let is_tracked = database::DB
                    .get_series(series_id)
                    .map(|series| {
                        if let Some(season) = series.get_season(episode_information.season) {
                            season.is_episode_watched(episode_information.number.unwrap())
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);

                checkbox("", is_tracked)
                    .on_toggle(move |_| Message::MarkedWatched(poster_type))
                    .size(17)
                    .into()
            }
        };

        row![
            text(format!(
                "{} {}",
                episode_information
                    .number
                    .map(|number| season_episode_str_gen(episode_information.season, number))
                    .unwrap_or_default(),
                episode_information.name
            ))
            .font(Font {
                weight: Weight::Bold,
                ..Default::default()
            })
            .style(styles::text_styles::accent_color_theme)
            .width(Length::FillPortion(10)),
            mark_watched_widget
        ]
        .spacing(5)
    }
}

pub mod series_poster {
    use std::borrow::Cow;
    use std::sync::mpsc;

    use crate::core::api::tv_maze::series_information::{Rating, SeriesMainInformation};
    use crate::core::api::tv_maze::Image;
    use crate::core::caching;
    use crate::core::posters_hiding::HIDDEN_SERIES;
    use crate::gui::assets::icons::{EYE_SLASH_FILL, STAR_FILL};
    use crate::gui::helpers;
    pub use crate::gui::message::IndexedMessage;
    use crate::gui::styles;

    use bytes::Bytes;
    use iced::font::Weight;
    use iced::widget::{button, column, container, image, mouse_area, row, svg, text, Space};
    use iced::{Element, Font, Task};

    #[derive(Debug, Clone)]
    pub enum GenericPosterMessage {
        ImageLoaded(Option<Bytes>),
    }

    pub struct GenericPoster<'a> {
        series_information: Cow<'a, SeriesMainInformation>,
        image: Option<Bytes>,
        series_page_sender: mpsc::Sender<SeriesMainInformation>,
    }

    impl<'a> GenericPoster<'a> {
        pub fn new(
            series_information: Cow<'a, SeriesMainInformation>,
            series_page_sender: mpsc::Sender<SeriesMainInformation>,
        ) -> (Self, Task<GenericPosterMessage>) {
            let image_url = series_information.image.clone();

            let poster = Self {
                series_information,
                image: None,
                series_page_sender,
            };

            (poster, Self::load_image(image_url))
        }

        pub fn update(&mut self, message: GenericPosterMessage) {
            match message {
                GenericPosterMessage::ImageLoaded(image) => self.image = image,
            }
        }

        pub fn get_series_info(&self) -> &SeriesMainInformation {
            &self.series_information
        }

        pub fn open_series_page(&self) {
            let series = self.series_information.clone().into_owned();
            self.series_page_sender
                .send(series)
                .expect("failed to send series page info");
        }

        pub fn get_image(&self) -> Option<&Bytes> {
            self.image.as_ref()
        }

        fn load_image(image: Option<Image>) -> Task<GenericPosterMessage> {
            if let Some(image) = image {
                Task::perform(
                    async move {
                        caching::load_image(
                            image.medium_image_url,
                            caching::ImageResolution::Medium,
                        )
                        .await
                    },
                    GenericPosterMessage::ImageLoaded,
                )
            } else {
                Task::none()
            }
        }
    }

    #[derive(Clone, Debug)]
    pub enum Message {
        Poster(GenericPosterMessage),
        SeriesPosterPressed,
        Expand,
        Hide,
        SeriesHidden,
    }

    pub struct SeriesPoster<'a> {
        index: usize,
        poster: GenericPoster<'a>,
        expanded: bool,
        hidden: bool,
    }

    impl<'a> SeriesPoster<'a> {
        pub fn new(
            index: usize,
            series_information: Cow<'a, SeriesMainInformation>,
            series_page_sender: mpsc::Sender<SeriesMainInformation>,
        ) -> (Self, Task<IndexedMessage<usize, Message>>) {
            let (poster, poster_command) =
                GenericPoster::new(series_information, series_page_sender);
            let poster = Self {
                index,
                poster,
                expanded: false,
                hidden: false,
            };

            (
                poster,
                poster_command
                    .map(Message::Poster)
                    .map(move |message| IndexedMessage::new(index, message)),
            )
        }

        pub fn get_series_info(&self) -> &SeriesMainInformation {
            self.poster.get_series_info()
        }

        pub fn update(
            &mut self,
            message: IndexedMessage<usize, Message>,
        ) -> Task<IndexedMessage<usize, Message>> {
            match message.message() {
                Message::SeriesPosterPressed => {
                    self.poster.open_series_page();
                }
                Message::Expand => self.expanded = !self.expanded,
                Message::Hide => {
                    let series_id = self.poster.get_series_info().id;
                    let index = self.index;
                    let series_name = self.poster.get_series_info().name.clone();
                    let premiered_date = self.poster.get_series_info().premiered.clone();

                    return Task::perform(
                        async move {
                            let mut hidden_series = HIDDEN_SERIES.write().await;

                            hidden_series
                                .hide_series(series_id, series_name, premiered_date)
                                .await
                        },
                        |_| Message::SeriesHidden,
                    )
                    .map(move |message| IndexedMessage::new(index, message));
                }
                Message::SeriesHidden => {
                    self.hidden = true;
                }
                Message::Poster(message) => self.poster.update(message),
            }
            Task::none()
        }

        pub fn is_hidden(&self) -> bool {
            self.hidden
        }

        pub fn view(&self, expandable: bool) -> Element<'_, IndexedMessage<usize, Message>> {
            let poster_image: Element<'_, Message> = {
                let image_height = if self.expanded { 170 } else { 140 };
                if let Some(image_bytes) = self.poster.get_image() {
                    let image_handle = image::Handle::from_bytes(image_bytes.clone());
                    image(image_handle).height(image_height).into()
                } else {
                    helpers::empty_image::empty_image()
                        .width(image_height as f32 / 1.4)
                        .height(image_height)
                        .into()
                }
            };

            let content: Element<'_, Message> = if self.expanded {
                let metadata = column![
                    text(&self.poster.get_series_info().name)
                        .size(11)
                        .font(Font {
                            weight: Weight::Bold,
                            ..Default::default()
                        })
                        .style(styles::text_styles::accent_color_theme),
                    Self::genres_widget(&self.poster.get_series_info().genres),
                    Self::premier_widget(self.poster.get_series_info().premiered.as_deref()),
                    Self::rating_widget(&self.poster.get_series_info().rating),
                    Space::with_height(5),
                    Self::hiding_button(),
                ]
                .spacing(2);

                row![poster_image, metadata]
                    .padding(2)
                    .spacing(5)
                    .width(300)
                    .into()
            } else {
                let mut content = column![].padding(2).spacing(1);
                content = content.push(poster_image);
                content = content.push(
                    text(&self.poster.get_series_info().name)
                        .size(11)
                        .width(100)
                        .height(30)
                        .align_x(iced::Alignment::Center)
                        .align_y(iced::Alignment::Center),
                );
                content.into()
            };

            let content = container(content)
                .padding(5)
                .style(styles::container_styles::second_class_container_rounded_theme);

            let mut mouse_area = mouse_area(content).on_press(Message::SeriesPosterPressed);

            if expandable {
                mouse_area = mouse_area.on_right_press(Message::Expand);
            }

            let element: Element<'_, Message> = mouse_area.into();
            element.map(|message| IndexedMessage::new(self.index, message))
        }

        fn rating_widget(rating: &Rating) -> Element<'_, Message> {
            if let Some(average_rating) = rating.average {
                let star_handle = svg::Handle::from_memory(STAR_FILL);
                let star_icon = svg(star_handle)
                    .width(15)
                    .height(15)
                    .style(styles::svg_styles::colored_svg_theme);

                row![star_icon, text(average_rating).size(11)]
                    .spacing(5)
                    .into()
            } else {
                Space::new(0, 0).into()
            }
        }

        fn premier_widget(premier_date: Option<&str>) -> Element<'_, Message> {
            if let Some(premier_date) = premier_date {
                text(format!("Premiered: {}", premier_date)).size(11).into()
            } else {
                Space::new(0, 0).into()
            }
        }

        fn genres_widget(genres: &[String]) -> Element<'_, Message> {
            if genres.is_empty() {
                Space::new(0, 0).into()
            } else {
                text(helpers::genres_with_pipes(genres)).size(11).into()
            }
        }

        fn hiding_button() -> Element<'static, Message> {
            let tracked_icon_handle = svg::Handle::from_memory(EYE_SLASH_FILL);
            let icon = svg(tracked_icon_handle)
                .width(15)
                .height(15)
                .style(styles::svg_styles::colored_svg_theme);

            let content = row![icon, text("Hide from Discover").size(11)].spacing(5);

            button(content)
                .on_press(Message::Hide)
                .style(styles::button_styles::transparent_button_with_rounded_border_theme)
                .into()
        }
    }
}

pub mod title_bar {
    use iced::widget::{
        button, container, horizontal_space, mouse_area, row, svg, text, Row, Space,
    };
    use iced::{Element, Length};

    use crate::gui::assets::icons::CARET_LEFT_FILL;
    use crate::gui::styles;
    use crate::gui::tabs::TabLabel;

    #[derive(Clone, Debug)]
    pub enum Message {
        TabSelected(usize),
        BackButtonPressed,
    }

    pub struct TitleBar {
        active_tab: usize,
    }

    impl TitleBar {
        pub fn new() -> Self {
            Self {
                active_tab: usize::default(),
            }
        }

        pub fn update(&mut self, message: Message) {
            if let Message::TabSelected(new_active_tab) = message {
                self.active_tab = new_active_tab
            }
        }

        pub fn view(
            &self,
            tab_labels: &[TabLabel],
            show_back_button: bool,
        ) -> iced::Element<'_, Message> {
            let tab_views = tab_labels.iter().enumerate().map(|(index, tab_label)| {
                let svg_handle = svg::Handle::from_memory(tab_label.icon);
                let icon = svg(svg_handle)
                    .width(Length::Shrink)
                    .style(styles::svg_styles::colored_svg_theme);
                let text_label = text(tab_label.text);
                let mut tab = container(
                    mouse_area(row![icon, text_label].spacing(5))
                        .on_press(Message::TabSelected(index)),
                )
                .padding(5);

                // Highlighting the tab if is active
                if index == self.active_tab {
                    tab = tab.style(styles::container_styles::second_class_container_square_theme)
                };
                tab.into()
            });

            let tab_views = Row::with_children(tab_views).spacing(10);

            let back_button: Element<'_, Message> = if show_back_button {
                let back_button_icon_handle = svg::Handle::from_memory(CARET_LEFT_FILL);
                let icon = svg(back_button_icon_handle)
                    .width(20)
                    .style(styles::svg_styles::colored_svg_theme);
                button(icon)
                    .on_press(Message::BackButtonPressed)
                    .style(styles::button_styles::transparent_button_theme)
                    .into()
            } else {
                Space::new(0, 0).into()
            };

            container(row![
                back_button,
                horizontal_space(),
                tab_views,
                horizontal_space()
            ])
            .style(styles::container_styles::first_class_container_square_theme)
            .into()
        }
    }
}
