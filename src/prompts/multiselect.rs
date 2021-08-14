use std::{collections::HashSet, iter::FromIterator};

use crate::{
    config::{self, Filter},
    error::{InquireError, InquireResult},
    formatter::{self, MultiOptionFormatter},
    input::Input,
    option_answer::OptionAnswer,
    ui::{
        crossterm::CrosstermTerminal, Backend, Key, KeyModifiers, MultiSelectBackend, RenderConfig,
    },
    utils::paginate,
    validator::MultiOptionValidator,
};

/// Prompt suitable for when you need the user to select many options (including none if applicable) among a list of them.
///
/// The user can select (or deselect) the current highlighted option by pressing space, clean all selections by pressing the left arrow and select all options by pressing the right arrow.
///
/// This prompt requires a prompt message and a **non-empty** list of options to be displayed to the user. If the list is empty, the prompt operation will fail with an [`InquireError::InvalidConfiguration`] error.
///
/// The options are paginated in order to provide a smooth experience to the user, with the default page size being 7. The user can move from the options and the pages will be updated accordingly, including moving from the last to the first options (or vice-versa).
///
/// Customizable options:
///
/// - **Prompt message**: Required when creating the prompt.
/// - **Options list**: Options displayed to the user. Must be **non-empty**.
/// - **Default selections**: Options that are selected by default when the prompt is first rendered. The user can unselect them. If any of the indices is out-of-range of the option list, the prompt will fail with an [`InquireError::InvalidConfiguration`] error.
/// - **Starting cursor**: Index of the cursor when the prompt is first rendered. Default is 0 (first option). If the index is out-of-range of the option list, the prompt will fail with an [`InquireError::InvalidConfiguration`] error.
/// - **Help message**: Message displayed at the line below the prompt.
/// - **Formatter**: Custom formatter in case you need to pre-process the user input before showing it as the final answer.
///   - Prints the selected options string value, joined using a comma as the separator, by default.
/// - **Validator**: Custom validator to make sure a given submitted input pass the specified requirements, e.g. not allowing 0 selected options or limiting the number of options that the user is allowed to select.
///   - No validators are on by default.
/// - **Page size**: Number of options displayed at once, 7 by default.
/// - **Filter function**: Function that defines if an option is displayed or not based on the current filter input.
/// - **Keep filter flag**: Whether the current filter input should be cleared or not after a selection is made. Defaults to true.
///
/// # Example
///
/// For a full-featured example, check the [GitHub repository](https://github.com/mikaelmello/inquire/blob/main/examples/multiselect.rs).
///
/// [`InquireError::InvalidConfiguration`]: crate::error::InquireError::InvalidConfiguration
#[derive(Copy, Clone)]
pub struct MultiSelect<'a> {
    /// Message to be presented to the user.
    pub message: &'a str,

    /// Options displayed to the user.
    pub options: &'a [&'a str],

    /// Default indexes of options to be selected from the start.
    pub default: Option<&'a [usize]>,

    /// Help message to be presented to the user.
    pub help_message: Option<&'a str>,

    /// Page size of the options displayed to the user.
    pub page_size: usize,

    /// Whether vim mode is enabled. When enabled, the user can
    /// navigate through the options using hjkl.
    pub vim_mode: bool,

    /// Starting cursor index of the selection.
    pub starting_cursor: usize,

    /// Function called with the current user input to filter the provided
    /// options.
    pub filter: Filter<'a>,

    /// Whether the current filter typed by the user is kept or cleaned after a selection is made.
    pub keep_filter: bool,

    /// Function that formats the user input and presents it to the user as the final rendering of the prompt.
    pub formatter: MultiOptionFormatter<'a>,

    /// Validator to apply to the user input.
    ///
    /// In case of error, the message is displayed one line above the prompt.
    pub validator: Option<MultiOptionValidator<'a>>,

    /// RenderConfig to apply to the rendered interface.
    pub render_config: RenderConfig,
}

impl<'a> MultiSelect<'a> {
    /// Default formatter, set to [DEFAULT_MULTI_OPTION_FORMATTER](crate::formatter::DEFAULT_MULTI_OPTION_FORMATTER)
    pub const DEFAULT_FORMATTER: MultiOptionFormatter<'a> =
        formatter::DEFAULT_MULTI_OPTION_FORMATTER;

    /// Default filter, equal to the global default filter [config::DEFAULT_FILTER].
    pub const DEFAULT_FILTER: Filter<'a> = config::DEFAULT_FILTER;

    /// Default page size, equal to the global default page size [config::DEFAULT_PAGE_SIZE]
    pub const DEFAULT_PAGE_SIZE: usize = config::DEFAULT_PAGE_SIZE;

    /// Default value of vim mode, equal to the global default value [config::DEFAULT_PAGE_SIZE]
    pub const DEFAULT_VIM_MODE: bool = config::DEFAULT_VIM_MODE;

    /// Default starting cursor index.
    pub const DEFAULT_STARTING_CURSOR: usize = 0;

    /// Default behavior of keeping or cleaning the current filter value.
    pub const DEFAULT_KEEP_FILTER: bool = true;

    /// Default help message.
    pub const DEFAULT_HELP_MESSAGE: Option<&'a str> =
        Some("↑↓ to move, space to select one, → to all, ← to none, type to filter");

    /// Default validator set for the [MultiSelect] prompt, none.
    pub const DEFAULT_VALIDATOR: Option<MultiOptionValidator<'a>> = None;

    /// Creates a [MultiSelect] with the provided message and options, along with default configuration values.
    pub fn new(message: &'a str, options: &'a [&str]) -> Self {
        Self {
            message,
            options,
            default: None,
            help_message: Self::DEFAULT_HELP_MESSAGE,
            page_size: Self::DEFAULT_PAGE_SIZE,
            vim_mode: Self::DEFAULT_VIM_MODE,
            starting_cursor: Self::DEFAULT_STARTING_CURSOR,
            keep_filter: Self::DEFAULT_KEEP_FILTER,
            filter: Self::DEFAULT_FILTER,
            formatter: Self::DEFAULT_FORMATTER,
            validator: Self::DEFAULT_VALIDATOR,
            render_config: RenderConfig::default(),
        }
    }

    /// Sets the help message of the prompt.
    pub fn with_help_message(mut self, message: &'a str) -> Self {
        self.help_message = Some(message);
        self
    }

    /// Removes the set help message.
    pub fn without_help_message(mut self) -> Self {
        self.help_message = None;
        self
    }

    /// Sets the page size.
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size;
        self
    }

    /// Enables or disabled vim_mode.
    pub fn with_vim_mode(mut self, vim_mode: bool) -> Self {
        self.vim_mode = vim_mode;
        self
    }

    /// Sets the keep filter behavior.
    pub fn with_keep_filter(mut self, keep_filter: bool) -> Self {
        self.keep_filter = keep_filter;
        self
    }

    /// Sets the filter function.
    pub fn with_filter(mut self, filter: Filter<'a>) -> Self {
        self.filter = filter;
        self
    }

    /// Sets the formatter.
    pub fn with_formatter(mut self, formatter: MultiOptionFormatter<'a>) -> Self {
        self.formatter = formatter;
        self
    }

    /// Sets the validator to apply to the user input. You might want to use this feature
    /// in case you need to limit the user to specific choices, such as limiting the number
    /// of selections.
    ///
    /// In case of error, the message is displayed one line above the prompt.
    pub fn with_validator(mut self, validator: MultiOptionValidator<'a>) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Sets the indexes to be selected by the default.
    pub fn with_default(mut self, default: &'a [usize]) -> Self {
        self.default = Some(default);
        self
    }

    /// Sets the starting cursor index.
    pub fn with_starting_cursor(mut self, starting_cursor: usize) -> Self {
        self.starting_cursor = starting_cursor;
        self
    }

    /// Sets the provided color theme to this prompt.
    pub fn with_render_config(mut self, render_config: RenderConfig) -> Self {
        self.render_config = render_config;
        self
    }

    /// Parses the provided behavioral and rendering options and prompts
    /// the CLI user for input according to the defined rules.
    pub fn prompt(self) -> InquireResult<Vec<OptionAnswer>> {
        let terminal = CrosstermTerminal::new()?;
        let mut backend = Backend::new(terminal, self.render_config)?;
        self.prompt_with_backend(&mut backend)
    }

    pub(in crate) fn prompt_with_backend<B: MultiSelectBackend>(
        self,
        backend: &mut B,
    ) -> InquireResult<Vec<OptionAnswer>> {
        MultiSelectPrompt::new(self)?.prompt(backend)
    }
}

struct MultiSelectPrompt<'a> {
    message: &'a str,
    options: &'a [&'a str],
    help_message: Option<&'a str>,
    vim_mode: bool,
    cursor_index: usize,
    checked: HashSet<usize>,
    page_size: usize,
    keep_filter: bool,
    input: Input,
    filtered_options: Vec<usize>,
    filter: Filter<'a>,
    formatter: MultiOptionFormatter<'a>,
    validator: Option<MultiOptionValidator<'a>>,
    error: Option<String>,
}

impl<'a> MultiSelectPrompt<'a> {
    fn new(mso: MultiSelect<'a>) -> InquireResult<Self> {
        if mso.options.is_empty() {
            return Err(InquireError::InvalidConfiguration(
                "Available options can not be empty".into(),
            ));
        }
        if let Some(default) = mso.default {
            for i in default {
                if i >= &mso.options.len() {
                    return Err(InquireError::InvalidConfiguration(format!(
                        "Index {} is out-of-bounds for length {} of options",
                        i,
                        &mso.options.len()
                    )));
                }
            }
        }

        Ok(Self {
            message: mso.message,
            options: mso.options,
            help_message: mso.help_message,
            vim_mode: mso.vim_mode,
            cursor_index: mso.starting_cursor,
            page_size: mso.page_size,
            keep_filter: mso.keep_filter,
            input: Input::new(),
            filtered_options: Vec::from_iter(0..mso.options.len()),
            filter: mso.filter,
            formatter: mso.formatter,
            validator: mso.validator,
            error: None,
            checked: mso
                .default
                .map_or_else(|| HashSet::new(), |d| d.iter().cloned().collect()),
        })
    }

    fn filter_options(&self) -> Vec<usize> {
        self.options
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| match self.input.content() {
                val if val.is_empty() => Some(i),
                val if (self.filter)(&val, opt, i) => Some(i),
                _ => None,
            })
            .collect()
    }

    fn move_cursor_up(&mut self) {
        self.cursor_index = self
            .cursor_index
            .checked_sub(1)
            .or(self.filtered_options.len().checked_sub(1))
            .unwrap_or_else(|| 0);
    }

    fn move_cursor_down(&mut self) {
        self.cursor_index = self.cursor_index.saturating_add(1);
        if self.cursor_index >= self.filtered_options.len() {
            self.cursor_index = 0;
        }
    }

    fn toggle_cursor_selection(&mut self) {
        let idx = match self.filtered_options.get(self.cursor_index) {
            Some(val) => val,
            None => return,
        };

        if self.checked.contains(idx) {
            self.checked.remove(idx);
        } else {
            self.checked.insert(*idx);
        }

        if !self.keep_filter {
            self.input.clear();
        }
    }

    fn on_change(&mut self, key: Key) {
        match key {
            Key::Up(KeyModifiers::NONE) => self.move_cursor_up(),
            Key::Char('k', KeyModifiers::NONE) if self.vim_mode => self.move_cursor_up(),
            Key::Down(KeyModifiers::NONE) => self.move_cursor_down(),
            Key::Char('j', KeyModifiers::NONE) if self.vim_mode => self.move_cursor_down(),
            Key::Char(' ', KeyModifiers::NONE) => self.toggle_cursor_selection(),
            Key::Right(KeyModifiers::NONE) => {
                self.checked.clear();
                for idx in &self.filtered_options {
                    self.checked.insert(*idx);
                }

                if !self.keep_filter {
                    self.input.clear();
                }
            }
            Key::Left(KeyModifiers::NONE) => {
                self.checked.clear();

                if !self.keep_filter {
                    self.input.clear();
                }
            }
            key => {
                let dirty = self.input.handle_key(key);

                if dirty {
                    let options = self.filter_options();
                    if options.len() > 0 && options.len() <= self.cursor_index {
                        self.cursor_index = options.len().saturating_sub(1);
                    }
                    self.filtered_options = options;
                }
            }
        };
    }

    fn get_final_answer(&self) -> Result<Vec<OptionAnswer>, String> {
        let selected_options = self
            .options
            .iter()
            .enumerate()
            .filter_map(|(idx, opt)| match &self.checked.contains(&idx) {
                true => Some(OptionAnswer::new(idx, opt)),
                false => None,
            })
            .collect::<Vec<OptionAnswer>>();

        if let Some(validator) = self.validator {
            return match validator(&selected_options) {
                Ok(_) => Ok(selected_options),
                Err(err) => Err(err),
            };
        }

        return Ok(selected_options);
    }

    fn render<B: MultiSelectBackend>(&mut self, backend: &mut B) -> InquireResult<()> {
        let prompt = &self.message;

        backend.frame_setup()?;

        if let Some(err) = &self.error {
            backend.render_error_message(err)?;
        }

        backend.render_multiselect_prompt(&prompt, &self.input)?;

        let choices = self
            .filtered_options
            .iter()
            .cloned()
            .map(|i| OptionAnswer::new(i, self.options.get(i).unwrap()))
            .collect::<Vec<OptionAnswer>>();

        let page = paginate(self.page_size, &choices, self.cursor_index);

        for (idx, opt) in page.content.iter().enumerate() {
            backend.render_option(
                &opt.value,
                page.selection == idx,
                self.checked.contains(&opt.index),
            )?;
        }

        if let Some(help_message) = self.help_message {
            backend.render_help_message(help_message)?;
        }

        backend.frame_finish()?;

        Ok(())
    }

    fn prompt<B: MultiSelectBackend>(
        mut self,
        backend: &mut B,
    ) -> InquireResult<Vec<OptionAnswer>> {
        let final_answer: Vec<OptionAnswer>;

        loop {
            self.render(backend)?;

            let key = backend.read_key()?;

            match key {
                Key::Cancel => return Err(InquireError::OperationCanceled),
                Key::Submit => match self.get_final_answer() {
                    Ok(answer) => {
                        final_answer = answer;
                        break;
                    }
                    Err(err) => self.error = Some(err),
                },
                key => self.on_change(key),
            }
        }

        let formatted = (self.formatter)(&final_answer);

        backend.finish_prompt(&self.message, &formatted)?;

        Ok(final_answer)
    }
}
