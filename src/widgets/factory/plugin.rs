use crate::application::plugin::Plugin;
use crate::widgets::factory::list::ListFactoryInit;
use adw::prelude::{ExpanderRowExt, PreferencesRowExt};
use proto_rust::provider::List;
use proto_rust::provider_client::ProviderClient;
use proto_rust::Channel;
use relm4::factory::AsyncFactoryComponent;
use relm4::factory::AsyncFactoryVecDeque;
use relm4::factory::{AsyncFactorySender, DynamicIndex, FactoryView};
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::ComponentController;
use relm4::{adw, Component, Controller};

use crate::widgets::components::list_entry::{ListEntryModel, ListEntryOutput};
use crate::widgets::components::sidebar::SidebarComponentInput;
use crate::widgets::factory::list::ListFactoryModel;

#[allow(dead_code)]
#[derive(Debug)]
pub struct PluginFactoryModel {
	pub plugin: Plugin,
	pub service: Option<ProviderClient<Channel>>,
	pub enabled: bool,
	pub lists: Vec<String>,
	pub list_factory: AsyncFactoryVecDeque<ListFactoryModel>,
	pub new_list_controller: Controller<ListEntryModel>,
}

#[derive(Debug)]
pub enum PluginFactoryInput {
	RequestAddList(usize, String),
	AddList(List),
	DeleteTaskList(DynamicIndex, String),
	Forward,
	ListSelected(ListFactoryModel),
	Notify(String),
	Enable,
	Disable,
}

#[derive(Debug)]
pub enum PluginFactoryOutput {
	AddListToProvider(usize, String, String),
	ListSelected(ListFactoryModel),
	Notify(String),
	Forward,
}

#[derive(derive_new::new)]
pub struct PluginFactoryInit {
	plugin: Plugin,
	enabled: bool,
}

#[relm4::factory(pub async)]
impl AsyncFactoryComponent for PluginFactoryModel {
	type ParentInput = SidebarComponentInput;
	type ParentWidget = adw::PreferencesGroup;
	type CommandOutput = ();
	type Input = PluginFactoryInput;
	type Output = PluginFactoryOutput;
	type Init = PluginFactoryInit;

	view! {
		#[root]
		adw::ExpanderRow {
			#[watch]
			set_title: self.plugin.name.as_str(),
			#[watch]
			set_subtitle: self.plugin.description.as_str(),
			#[watch]
			set_icon_name: Some(self.plugin.icon.as_str()),
			#[watch]
			set_enable_expansion: !self.lists.is_empty() && self.plugin.is_running() && self.enabled,
			set_expanded: !self.lists.is_empty(),
			add_action = if self.plugin.is_running() {
				gtk::MenuButton {
					set_icon_name: "value-increase-symbolic",
					set_css_classes: &["flat", "image-button"],
					set_valign: gtk::Align::Center,
					set_direction: gtk::ArrowType::Right,
					set_popover: Some(self.new_list_controller.widget())
				}
			} else {
				gtk::Box {

				}
			},
		}
	}

	async fn init_model(
		init: Self::Init,
		index: &DynamicIndex,
		sender: AsyncFactorySender<Self>,
	) -> Self {
		let index = index.current_index();
		Self {
			plugin: init.plugin.clone(),
			service: init.plugin.connect().await.ok(),
			enabled: init.enabled,
			lists: init.plugin.lists().await.unwrap(),
			list_factory: AsyncFactoryVecDeque::new(
				adw::ExpanderRow::default(),
				sender.input_sender(),
			),
			new_list_controller: ListEntryModel::builder().launch(()).forward(
				sender.input_sender(),
				move |message| match message {
					ListEntryOutput::AddTaskListToSidebar(name) => {
						PluginFactoryInput::RequestAddList(index, name)
					},
				},
			),
		}
	}

	fn init_widgets(
		&mut self,
		_index: &DynamicIndex,
		root: &Self::Root,
		_returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
		sender: AsyncFactorySender<Self>,
	) -> Self::Widgets {
		let widgets = view_output!();

		self.list_factory =
			AsyncFactoryVecDeque::new(root.clone(), sender.input_sender());

		for list in &self.lists {
			self
				.list_factory
				.guard()
				.push_back(ListFactoryInit::new(list.clone(), self.service.clone()));
		}

		widgets
	}

	async fn update(
		&mut self,
		message: Self::Input,
		sender: AsyncFactorySender<Self>,
	) {
		match message {
			PluginFactoryInput::DeleteTaskList(index, list_id) => {
				self.list_factory.guard().remove(index.current_index());
				let index = self
					.lists
					.iter()
					.position(|list_id| list_id.eq(list_id))
					.unwrap();
				self.lists.remove(index);
				info!("Deleted task list with id: {}", list_id);
			},
			PluginFactoryInput::RequestAddList(index, name) => {
				sender.output(PluginFactoryOutput::AddListToProvider(
					index,
					self.plugin.id.clone(),
					name,
				))
			},
			PluginFactoryInput::AddList(list) => {
				self.list_factory.guard().push_back(ListFactoryInit::new(
					list.id.clone(),
					self.service.clone(),
				));
				self.lists.push(list.id);
				info!("List added to {}", self.plugin.name);
			},
			PluginFactoryInput::Forward => {
				sender.output(PluginFactoryOutput::Forward)
			},
			PluginFactoryInput::ListSelected(model) => {
				sender.output(PluginFactoryOutput::ListSelected(model.clone()));
				info!("List selected: {}", model.list.unwrap().name);
			},
			PluginFactoryInput::Notify(msg) => {
				sender.output(PluginFactoryOutput::Notify(msg))
			},
			PluginFactoryInput::Enable => {
				self.enabled = true;

				self.list_factory.guard().clear();
				for list in &self.lists {
					self.list_factory.guard().push_back(ListFactoryInit::new(
						list.clone(),
						self.service.clone(),
					));
				}
			},
			PluginFactoryInput::Disable => self.enabled = false,
		}
	}

	fn output_to_parent_input(output: Self::Output) -> Option<Self::ParentInput> {
		let output = match output {
			PluginFactoryOutput::ListSelected(list) => {
				SidebarComponentInput::ListSelected(list)
			},
			PluginFactoryOutput::Forward => SidebarComponentInput::Forward,
			PluginFactoryOutput::AddListToProvider(index, provider_id, name) => {
				SidebarComponentInput::AddListToProvider(index, provider_id, name)
			},
			PluginFactoryOutput::Notify(msg) => SidebarComponentInput::Notify(msg),
		};
		Some(output)
	}
}
