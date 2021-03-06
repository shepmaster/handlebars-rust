use std::collections::BTreeMap;
use std::iter::FromIterator;

use registry::Registry;
use render::{RenderError, RenderContext, Directive, Evaluable, Renderable};

pub fn expand_partial(d: &Directive,
                      r: &Registry,
                      rc: &mut RenderContext)
                      -> Result<(), RenderError> {

    // try eval inline partials first
    if let Some(t) = d.template() {
        try!(t.eval(r, rc));
    }

    if rc.is_current_template(d.name()) {
        return Err(RenderError::new("Cannot include self in >"));
    }


    let tname = d.name();
    let partial = rc.get_partial(tname);
    let render_template = partial.as_ref().or(r.get_template(tname)).or(d.template());
    match render_template {
        Some(t) => {
            let context_param = d.params().get(0).and_then(|p| p.path());
            let old_path = match context_param {
                Some(p) => {
                    let old_path = rc.get_path().clone();
                    rc.promote_local_vars();
                    let new_path = format!("{}/{}", old_path, p);
                    rc.set_path(new_path);
                    Some(old_path)
                }
                None => None,
            };

            let hash = d.hash();
            let r = if hash.is_empty() {
                t.render(r, rc)
            } else {
                let hash_ctx = BTreeMap::from_iter(hash.iter()
                                                       .map(|(k, v)| {
                                                           (k.clone(), v.value().clone())
                                                       }));
                let mut local_rc = rc.derive();
                {
                    let mut ctx_ref = local_rc.context_mut();
                    *ctx_ref = ctx_ref.extend(&hash_ctx);
                }
                t.render(r, &mut local_rc)
            };

            if let Some(path) = old_path {
                rc.set_path(path);
                rc.demote_local_vars();
            }

            r
        }
        None => Ok(()),
    }

}

#[cfg(test)]
mod test {
    use registry::Registry;

    #[test]
    fn test() {
        let mut handlebars = Registry::new();
        assert!(handlebars.register_template_string("t0", "{{> t1}}").is_ok());
        assert!(handlebars.register_template_string("t1", "{{this}}").is_ok());
        assert!(handlebars.register_template_string("t2", "{{#> t99}}not there{{/t99}}").is_ok());
        assert!(handlebars.register_template_string("t3", "{{#*inline \"t31\"}}{{this}}{{/inline}}{{> t31}}").is_ok());
        assert!(handlebars.register_template_string("t4", "{{#> t5}}{{#*inline \"nav\"}}navbar{{/inline}}{{/t5}}").is_ok());
        assert!(handlebars.register_template_string("t5", "include {{> nav}}").is_ok());
        assert!(handlebars.register_template_string("t6", "{{> t1 a}}").is_ok());
        assert!(handlebars.register_template_string("t7", "{{#*inline \"t71\"}}{{a}}{{/inline}}{{> t71 a=\"world\"}}").is_ok());
        assert!(handlebars.register_template_string("t8", "{{a}}").is_ok());
        assert!(handlebars.register_template_string("t9", "{{> t8 a=2}}").is_ok());

        assert_eq!(handlebars.render("t0", &1).ok().unwrap(), "1".to_string());
        assert_eq!(handlebars.render("t2", &1).ok().unwrap(),
                   "not there".to_string());
        assert_eq!(handlebars.render("t3", &1).ok().unwrap(), "1".to_string());
        assert_eq!(handlebars.render("t4", &1).ok().unwrap(),
                   "include navbar".to_string());
        assert_eq!(handlebars.render("t6", &btreemap!{"a".to_string() => "2".to_string()})
                             .ok()
                             .unwrap(),
                   "2".to_string());
        assert_eq!(handlebars.render("t7", &1).ok().unwrap(),
                   "world".to_string());
        assert_eq!(handlebars.render("t9", &1).ok().unwrap(), "2".to_string());
    }
}
