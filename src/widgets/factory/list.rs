use proto_rust::provider_client::ProviderClient;
use proto_rust::Channel;
use relm4::adw::prelude::ActionRowExt;
use relm4::factory::AsyncFactoryComponent;
use relm4::factory::{DynamicIndex, FactoryView};
use relm4::{view, AsyncFactorySender};

use crate::gtk::prelude::{
	ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, WidgetExt,
};
use crate::widgets::factory::plugin::PluginFactoryInput;
use proto_rust::provider::List;
use relm4::loading_widgets::LoadingWidgets;

use crate::{adw, gtk};

#[derive(Debug, Clone)]
pub struct ListFactoryModel {
	pub list: Option<List>,
	pub tasks: Vec<String>,
	pub service: Option<ProviderClient<Channel>>,
}

#[derive(Debug)]
pub enum ListFactoryInput {
	Select,
	Delete(DynamicIndex),
	Rename(String),
	ChangeIcon(String),
}

#[derive(Debug)]
pub enum ListFactoryOutput {
	Select(ListFactoryModel),
	DeleteTaskList(DynamicIndex, String),
	Forward,
	Notify(String),
}

#[derive(derive_new::new)]
pub struct ListFactoryInit {
	list_id: String,
	service: Option<ProviderClient<Channel>>,
}

#[relm4::factory(pub async)]
impl AsyncFactoryComponent for ListFactoryModel {
	type ParentInput = PluginFactoryInput;
	type ParentWidget = adw::ExpanderRow;
	type CommandOutput = ();
	type Input = ListFactoryInput;
	type Output = ListFactoryOutput;
	type Init = ListFactoryInit;
	type Widgets = ListWidgets;

	view! {
		#[root]
		gtk::ListBoxRow {
			set_visible: self.list.is_some(),
			adw::ActionRow {
				add_prefix = &gtk::Entry {
					set_hexpand: false,
					add_css_class: "flat",
					add_css_class: "no-border",
					#[watch]
					set_text: if self.list.is_some() {
						self.list.as_ref().unwrap().name.as_str()
					} else {
						""
					},
					set_margin_top: 10,
					set_margin_bottom: 10,
					connect_activate[sender] => move |entry| {
						let buffer = entry.buffer();
						sender.input(ListFactoryInput::Rename(buffer.text()));
					},
					// This crashes the program.
					// connect_changed[sender] => move |entry| {
					// 	let buffer = entry.buffer();
					// 	sender.input(ListInput::Rename(buffer.text()));
					// }
				},
				add_prefix = &gtk::MenuButton {
					#[watch]
					set_label: if self.list.is_some() {
						self.list.as_ref().unwrap().icon.as_ref().unwrap().as_str()
					} else {
						""
					},
					set_css_classes: &["flat", "image-button"],
					set_valign: gtk::Align::Center,
					#[wrap(Some)]
					set_popover = &gtk::EmojiChooser{
						connect_emoji_picked[sender] => move |_, emoji| {
							sender.input(ListFactoryInput::ChangeIcon(emoji.to_string()));
						}
					}
				},
				add_suffix = &gtk::Label {
					set_halign: gtk::Align::End,
					set_css_classes: &["dim-label", "caption"],
					// #[watch]
					// set_text: self.count.to_string().as_str(),
				},
				add_suffix = &gtk::Button {
					set_icon_name: "user-trash-full-symbolic",
					set_css_classes: &["circular", "image-button", "destructive-action"],
					set_valign: gtk::Align::Center,
					connect_clicked[sender, index] => move |_| {
						sender.input(ListFactoryInput::Delete(index.clone()));
					}
				},
			},
			add_controller = &gtk::GestureClick {
				connect_pressed[sender] => move |_, _, _, _| {
					sender.input(ListFactoryInput::Select);
					sender.output(ListFactoryOutput::Forward);
				}
			}
		}
	}

	fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
		view! {
			#[local_ref]
			root {
				#[name(spinner)]
				adw::ActionRow {
					add_prefix = &gtk::Spinner {
						start: (),
						set_hexpand: false,
					}
				}
			}
		}
		Some(LoadingWidgets::new(root, spinner))
	}

	fn init_widgets(
		&mut self,
		index: &DynamicIndex,
		root: &Self::Root,
		_returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
		sender: AsyncFactorySender<Self>,
	) -> Self::Widgets {
		let widgets = view_output!();
		widgets
	}

	async fn init_model(
		params: Self::Init,
		_index: &DynamicIndex,
		_sender: AsyncFactorySender<Self>,
	) -> Self {
		let mut service = params.service.clone();
		let mut list = None;
		let mut tasks = vec![];
		if let Some(client) = &mut service {
			list = match client.read_list(params.list_id.clone()).await {
				Ok(response) => {
					let response = response.into_inner();
					if response.list.is_some() {
						tasks = match client
							.read_task_ids_from_list(params.list_id.clone())
							.await
						{
							Ok(response) => response.into_inner().tasks,
							Err(e) => {
								error!("Failed to find tasks. {:?}", e);
								vec![]
							},
						};
					}
					response.list
				},
				Err(e) => {
					error!("Failed to find list. {:?}", e);
					None
				},
			};
		}

		Self {
			list,
			tasks,
			service: params.service,
		}
	}

	async fn update(
		&mut self,
		message: Self::Input,
		sender: AsyncFactorySender<Self>,
	) {
		match message {
			ListFactoryInput::Rename(name) => {
				let mut list = self.list.clone().unwrap();
				list.name = name.clone();
				if let Some(client) = &mut self.service {
					match client.update_list(list).await {
						Ok(response) => {
							let response = response.into_inner();
							if response.successful {
								self.list.as_mut().unwrap().name = name;
							}
							sender.output(ListFactoryOutput::Notify(response.message));
						},
						Err(err) => {
							sender.output(ListFactoryOutput::Notify(err.to_string()))
						},
					}
				}
			},
			ListFactoryInput::Delete(index) => {
				let list_id = self.list.as_ref().unwrap().id.clone();
				if let Some(client) = &mut self.service {
					match client.delete_list(list_id.clone()).await {
						Ok(response) => {
							let response = response.into_inner();
							if response.successful {
								sender
									.output(ListFactoryOutput::DeleteTaskList(index, list_id));
							}
							sender.output(ListFactoryOutput::Notify(response.message));
						},
						Err(err) => {
							sender.output(ListFactoryOutput::Notify(err.to_string()))
						},
					}
				}
			},
			ListFactoryInput::ChangeIcon(icon) => {
				if let Some(client) = &mut self.service {
					let mut list = self.list.clone().unwrap();
					list.icon = Some(icon.clone());
					match client.update_list(list).await {
						Ok(response) => {
							let response = response.into_inner();
							if response.successful {
								self.list.as_mut().unwrap().icon = Some(icon);
							}
							sender.output(ListFactoryOutput::Notify(response.message));
						},
						Err(err) => {
							sender.output(ListFactoryOutput::Notify(err.to_string()))
						},
					}
				}
			},
			ListFactoryInput::Select => {
				if let Some(client) = &mut self.service {
					let tasks = match client
						.read_task_ids_from_list(self.list.as_ref().unwrap().id.clone())
						.await
					{
						Ok(response) => response.into_inner().tasks,
						Err(e) => {
							error!("Failed to find tasks. {:?}", e);
							vec![]
						},
					};
					self.tasks = tasks;
					sender.output(ListFactoryOutput::Select(self.clone()));
				}
			},
		}
	}

	fn output_to_parent_input(output: Self::Output) -> Option<Self::ParentInput> {
		match output {
			ListFactoryOutput::Select(data) => {
				Some(PluginFactoryInput::ListSelected(data))
			},
			ListFactoryOutput::DeleteTaskList(index, list_id) => {
				Some(PluginFactoryInput::DeleteTaskList(index, list_id))
			},
			ListFactoryOutput::Forward => Some(PluginFactoryInput::Forward),
			ListFactoryOutput::Notify(msg) => Some(PluginFactoryInput::Notify(msg)),
		}
	}
}
