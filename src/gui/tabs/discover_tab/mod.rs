use std::sync::mpsc;

use crate::core::api::tv_maze::series_information::SeriesMainInformation;
use crate::gui::assets::icons::BINOCULARS_FILL;
use crate::gui::styles;
use full_schedule::{FullSchedulePosters, Message as FullSchedulePostersMessage};
use searching::Message as SearchMessage;

use iced::widget::scrollable::{RelativeOffset, Viewport};
use iced::widget::{center, column, container, scrollable, stack, Space};
use iced::{Element, Length, Task};

use iced_aw::Spinner;

use super::Tab;

mod full_schedule;
mod searching;

#[derive(Clone, Debug)]
pub enum Message {
    Reload,
    FullSchedulePosters(FullSchedulePostersMessage),
    Search(SearchMessage),
    PageScrolled(Viewport),
}

pub struct DiscoverTab<'a> {
    search: searching::Search,
    full_schedule_series: FullSchedulePosters<'a>,
    scrollable_offset: RelativeOffset,
}

impl DiscoverTab<'_> {
    pub fn new(series_page_sender: mpsc::Sender<SeriesMainInformation>) -> (Self, Task<Message>) {
        let (full_schedule_series, full_schedule_command) =
            FullSchedulePosters::new(series_page_sender.clone());

        (
            Self {
                search: searching::Search::new(series_page_sender),
                full_schedule_series,
                scrollable_offset: RelativeOffset::START,
            },
            full_schedule_command.map(Message::FullSchedulePosters),
        )
    }

    pub fn refresh(&mut self) -> Task<Message> {
        self.full_schedule_series
            .refresh_daily_local_series()
            .map(Message::FullSchedulePosters)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch([
            iced::keyboard::on_key_press(|key, modifiers| {
                if key == iced::keyboard::key::Key::Named(iced::keyboard::key::Named::F5)
                    && modifiers.is_empty()
                {
                    Some(Message::Reload)
                } else {
                    None
                }
            }),
            self.search.subscription().map(Message::Search),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Reload => self
                .full_schedule_series
                .reload()
                .map(Message::FullSchedulePosters),
            Message::Search(message) => self.search.update(message).map(Message::Search),
            Message::FullSchedulePosters(message) => self
                .full_schedule_series
                .update(message)
                .map(Message::FullSchedulePosters),
            Message::PageScrolled(view_port) => {
                self.scrollable_offset = view_port.relative_offset();
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let underlay: Element<'_, Message> =
            if let Some(full_schedule_series) = self.full_schedule_series.view() {
                scrollable(full_schedule_series.map(Message::FullSchedulePosters))
                    .direction(styles::scrollable_styles::vertical_direction())
                    .id(Self::scrollable_id())
                    .on_scroll(Message::PageScrolled)
                    .width(Length::Fill)
                    .into()
            } else {
                center(Spinner::new()).into()
            };

        let overlay = self
            .search
            .view()
            .1
            .map(|element| element.map(Message::Search))
            .unwrap_or(Space::new(0, 0).into());

        let overlay = container(overlay).center_x(Length::Fill);

        let content = stack([underlay, overlay.into()]);

        column![self.search.view().0.map(Message::Search), content]
            .spacing(2)
            .into()
    }
}

impl Tab for DiscoverTab<'_> {
    type Message = Message;

    fn title() -> &'static str {
        "Discover"
    }

    fn icon_bytes() -> &'static [u8] {
        BINOCULARS_FILL
    }

    fn get_scrollable_offset(&self) -> RelativeOffset {
        self.scrollable_offset
    }
}
