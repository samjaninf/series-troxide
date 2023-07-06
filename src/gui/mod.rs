mod assets;
mod helpers;
mod troxide_widget;
mod view;

use troxide_widget::series_poster::Message as SeriesPosterMessage;
use view::discover_view::{DiscoverTab, Message as DiscoverMessage};
use view::my_shows_view::{Message as MyShowsMessage, MyShowsTab};
use view::series_view::Message as SeriesMessage;
use view::series_view::Series;
use view::settings_view::{Message as SettingsMessage, SettingsTab};
use view::statistics_view::{Message as StatisticsMessage, StatisticsTab};
use view::watchlist_view::{Message as WatchlistMessage, WatchlistTab};

use iced::widget::{container, text, Column};
use iced::{Application, Command, Element, Length};
use iced_aw::{TabLabel, Tabs};

use super::core::settings_config;

#[derive(Debug, Clone)]
enum TabId {
    Discover,
    Watchlist,
    MyShows,
    Statistics,
    Settings,
}

impl From<usize> for TabId {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Discover,
            1 => Self::Watchlist,
            2 => Self::MyShows,
            3 => Self::Statistics,
            4 => Self::Settings,
            _ => unreachable!("no more tabs"),
        }
    }
}

impl Into<usize> for TabId {
    fn into(self) -> usize {
        match self {
            TabId::Discover => 0,
            TabId::Watchlist => 1,
            TabId::MyShows => 2,
            TabId::Statistics => 3,
            TabId::Settings => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(usize),
    Discover(DiscoverMessage),
    Watchlist(WatchlistMessage),
    MyShows(MyShowsMessage),
    Statistics(StatisticsMessage),
    Settings(SettingsMessage),
    Series(SeriesMessage),
}

pub struct TroxideGui {
    active_tab: TabId,
    series_view_active: bool,
    discover_tab: DiscoverTab,
    watchlist_tab: WatchlistTab,
    my_shows_tab: MyShowsTab,
    statistics_tab: StatisticsTab,
    settings_tab: SettingsTab,
    series_view: Option<Series>,
}

impl Application for TroxideGui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = settings_config::Config;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let (discover_tab, discover_command) = view::discover_view::DiscoverTab::new();
        (
            Self {
                active_tab: TabId::Discover,
                series_view_active: false,
                discover_tab,
                watchlist_tab: WatchlistTab::default(),
                statistics_tab: StatisticsTab::default(),
                my_shows_tab: MyShowsTab::default(),
                settings_tab: view::settings_view::SettingsTab::new(flags),
                series_view: None,
            },
            discover_command.map(Message::Discover),
        )
    }

    fn title(&self) -> String {
        "Series Troxide".to_string()
    }

    fn theme(&self) -> iced::Theme {
        match self.settings_tab.get_config_settings().theme {
            settings_config::Theme::Light => iced::Theme::Light,
            settings_config::Theme::Dark => iced::Theme::Dark,
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if let Some((series_view, series_command)) =
            handle_series_poster_selection(&self.active_tab, message.clone())
        {
            self.series_view = Some(series_view);
            self.series_view_active = true;
            return series_command.map(Message::Series);
        }

        match message {
            Message::TabSelected(tab_id) => {
                self.series_view_active = false;
                let tab_id = TabId::from(tab_id);
                self.active_tab = tab_id.clone();
                if let TabId::MyShows = tab_id {
                    return self.my_shows_tab.refresh().map(Message::MyShows);
                };
                if let TabId::Watchlist = tab_id {
                    return self.watchlist_tab.refresh().map(Message::Watchlist);
                };
                if let TabId::Statistics = tab_id {
                    return self.statistics_tab.refresh().map(Message::Statistics);
                };
                Command::none()
            }
            Message::Discover(message) => self.discover_tab.update(message).map(Message::Discover),
            Message::Watchlist(message) => {
                self.watchlist_tab.update(message).map(Message::Watchlist)
            }
            Message::MyShows(message) => self.my_shows_tab.update(message).map(Message::MyShows),
            Message::Statistics(message) => {
                self.statistics_tab.update(message);
                Command::none()
            }
            Message::Settings(message) => {
                self.settings_tab.update(message);
                Command::none()
            }
            Message::Series(message) => {
                if let Some(command) =
                    handle_back_message_from_series(&message, &mut self.series_view_active)
                {
                    return command;
                };
                self.series_view
                    .as_mut()
                    .expect("for series view to send a message it must exist")
                    .update(message)
                    .map(Message::Series)
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Message, iced::Renderer<Self::Theme>> {
        let mut tabs: Vec<(TabLabel, Element<'_, Message, iced::Renderer>)> = vec![
            (
                self.discover_tab.tab_label(),
                self.discover_tab.view().map(Message::Discover),
            ),
            (
                self.watchlist_tab.tab_label(),
                self.watchlist_tab.view().map(Message::Watchlist),
            ),
            (
                self.my_shows_tab.tab_label(),
                self.my_shows_tab.view().map(Message::MyShows),
            ),
            (
                self.statistics_tab.tab_label(),
                self.statistics_tab.view().map(Message::Statistics),
            ),
            (
                self.settings_tab.tab_label(),
                self.settings_tab.view().map(Message::Settings),
            ),
        ];

        let active_tab_index = self.active_tab.to_owned().into();

        // Hijacking the current tab view when series view is active
        if self.series_view_active {
            let (_, current_view): &mut (TabLabel, Element<'_, Message, iced::Renderer>) =
                &mut tabs[active_tab_index];
            *current_view = self
                .series_view
                .as_ref()
                .unwrap()
                .view()
                .map(Message::Series);
        }

        Tabs::with_tabs(active_tab_index, tabs, Message::TabSelected).into()
    }
}

fn handle_series_poster_selection(
    tab_id: &TabId,
    message: Message,
) -> Option<(Series, Command<SeriesMessage>)> {
    match tab_id {
        TabId::Discover => {
            if let Message::Discover(message) = message {
                match message {
                    DiscoverMessage::SeriesSelected(series_info) => {
                        return Some(view::series_view::Series::from_series_information(
                            *series_info,
                        ));
                    }
                    DiscoverMessage::SeriesResultSelected(series_id) => {
                        return Some(view::series_view::Series::from_series_id(series_id));
                    }
                    _ => return None,
                }
            }
        }
        TabId::MyShows => {
            if let Message::MyShows(MyShowsMessage::SeriesSelected(series_info)) = message {
                return Some(view::series_view::Series::from_series_information(
                    *series_info,
                ));
            }
        }
        TabId::Watchlist => {
            if let Message::Watchlist(WatchlistMessage::SeriesPoster(
                _,
                SeriesPosterMessage::SeriesPosterPressed(series_info),
            )) = message
            {
                return Some(view::series_view::Series::from_series_information(
                    *series_info,
                ));
            }
        }
        _ => return None,
    }
    None
}

fn handle_back_message_from_series(
    series_message: &SeriesMessage,
    series_view_active: &mut bool,
) -> Option<Command<Message>> {
    if let SeriesMessage::GoBack = series_message {
        *series_view_active = false;
        return Some(Command::none());
    }
    None
}

trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&self) -> Element<'_, Self::Message> {
        let column = Column::new()
            .spacing(20)
            .push(text(self.title()).size(32))
            .push(self.content());

        container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn content(&self) -> Element<'_, Self::Message>;
}
