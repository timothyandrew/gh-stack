use dialoguer::Input;

pub fn loop_until_confirm(prompt: &str) {
  let prompt = format!("{} Type 'yes' to continue", prompt);
  loop {
      let result = Input::<String>::new()
          .with_prompt(&prompt)
          .interact()
          .unwrap();
      match &result[..] {
          "yes" => return,
          _ => continue,
      }
  }
}
