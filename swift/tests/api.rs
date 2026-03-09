use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use sudachi_swift::{
    SudachiDictionary, SudachiError, SudachiInfoField, SudachiPartOfSpeech,
    SudachiPartialPartOfSpeech, SudachiProjection, SudachiSplitMode,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("swift crate should live under repo root")
        .to_path_buf()
}

fn test_resource(path: &str) -> String {
    repo_root()
        .join("sudachi")
        .join("tests")
        .join("resources")
        .join(path)
        .to_string_lossy()
        .into_owned()
}

fn write_test_config(system_dict: &str, projection: Option<&str>) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("sudachi-swift-test-{nonce}.json"));
    let projection = projection
        .map(|value| format!(r#","projection":"{value}""#))
        .unwrap_or_default();
    let config = format!(
        r#"{{
  "systemDict":"{system_dict}",
  "userDict":["user.dic.test"],
  "characterDefinitionFile":"char.def",
  "oovProviderPlugin":[
    {{
      "class":"com.worksap.nlp.sudachi.SimpleOovPlugin",
      "oovPOS":["名詞","普通名詞","一般","*","*","*"],
      "leftId":0,
      "rightId":0,
      "cost":30000
    }}
  ]{projection}
}}"#
    );
    fs::write(&path, config).expect("test config should be written");
    path.to_string_lossy().into_owned()
}

fn test_dictionary(projection: Option<&str>) -> std::sync::Arc<SudachiDictionary> {
    SudachiDictionary::new(
        test_resource("system.dic.test"),
        Some(write_test_config("missing-system.dic", projection)),
        Some(test_resource("")),
    )
    .expect("test dictionary should load")
}

#[test]
fn constructor_requires_valid_config() {
    let err = match SudachiDictionary::new(
        test_resource("system.dic.test"),
        Some(test_resource("missing-config.json")),
        Some(test_resource("")),
    ) {
        Err(err) => err,
        Ok(_) => panic!("missing config path should fail"),
    };

    assert!(matches!(err, SudachiError::Config(_)));
}

#[test]
fn constructor_surfaces_dictionary_errors() {
    let err = match SudachiDictionary::new(
        test_resource("missing-system.dic"),
        Some(write_test_config("missing-system.dic", None)),
        Some(test_resource("")),
    ) {
        Err(err) => err,
        Ok(_) => panic!("missing dictionary path should fail"),
    };

    assert!(matches!(err, SudachiError::Dictionary(_)));
}

#[test]
fn dictionary_projection_default_and_override_work() {
    let dict = test_dictionary(Some("reading"));

    let default_tokenizer = dict.create_tokenizer(None, None, None);
    let default_list = default_tokenizer
        .tokenize("京都")
        .expect("default tokenization should succeed");
    let default_morpheme = default_list
        .morpheme_at(0)
        .expect("first morpheme should exist");
    assert_eq!(default_morpheme.surface(), "キョウト");
    assert_eq!(default_morpheme.raw_surface(), "京都");

    let surface_tokenizer = dict.create_tokenizer(None, None, Some(SudachiProjection::Surface));
    let surface_list = surface_tokenizer
        .tokenize("京都")
        .expect("surface tokenization should succeed");
    assert_eq!(
        surface_list
            .morpheme_at(0)
            .expect("first morpheme should exist")
            .surface(),
        "京都"
    );

    let dictionary_tokenizer =
        dict.create_tokenizer(None, None, Some(SudachiProjection::Dictionary));
    let dictionary_list = dictionary_tokenizer
        .tokenize("行っ")
        .expect("dictionary projection should succeed");
    assert_eq!(
        dictionary_list
            .morpheme_at(0)
            .expect("first morpheme should exist")
            .surface(),
        "行く"
    );

    let keep_surface_tokenizer =
        dict.create_tokenizer(None, None, Some(SudachiProjection::DictionaryAndSurface));
    let keep_surface_list = keep_surface_tokenizer
        .tokenize("行っ")
        .expect("dictionary_and_surface projection should succeed");
    assert_eq!(
        keep_surface_list
            .morpheme_at(0)
            .expect("first morpheme should exist")
            .surface(),
        "行っ"
    );

    let normalized_tokenizer =
        dict.create_tokenizer(None, None, Some(SudachiProjection::Normalized));
    let normalized_list = normalized_tokenizer
        .tokenize("行っ")
        .expect("normalized projection should succeed");
    assert_eq!(
        normalized_list
            .morpheme_at(0)
            .expect("first morpheme should exist")
            .surface(),
        "行く"
    );

    let normalized_and_surface_tokenizer =
        dict.create_tokenizer(None, None, Some(SudachiProjection::NormalizedAndSurface));
    let normalized_and_surface_list = normalized_and_surface_tokenizer
        .tokenize("行っ")
        .expect("normalized_and_surface projection should succeed");
    assert_eq!(
        normalized_and_surface_list
            .morpheme_at(0)
            .expect("first morpheme should exist")
            .surface(),
        "行っ"
    );

    let normalized_nouns_tokenizer =
        dict.create_tokenizer(None, None, Some(SudachiProjection::NormalizedNouns));
    let normalized_nouns_list = normalized_nouns_tokenizer
        .tokenize("行っ")
        .expect("normalized_nouns projection should succeed");
    assert_eq!(
        normalized_nouns_list
            .morpheme_at(0)
            .expect("first morpheme should exist")
            .surface(),
        "行っ"
    );
}

#[test]
fn tokenize_returns_morpheme_list_and_supports_split() {
    let dict = test_dictionary(None);
    let tokenizer = dict.create_tokenizer(Some(SudachiSplitMode::C), None, None);

    let list = tokenizer
        .tokenize("東京都")
        .expect("mode C tokenization should succeed");
    assert_eq!(list.count(), 1);
    assert!(!list.is_empty());
    assert_eq!(list.morphemes().len(), 1);

    let morpheme = list.morpheme_at(0).expect("first morpheme should exist");
    assert_eq!(morpheme.begin(), 0);
    assert_eq!(morpheme.end(), 3);
    assert_eq!(morpheme.length(), 3);
    assert_eq!(morpheme.surface(), "東京都");
    assert_eq!(morpheme.raw_surface(), "東京都");
    assert_eq!(morpheme.dictionary_form(), "東京都");
    assert_eq!(morpheme.normalized_form(), "東京都");
    assert_eq!(morpheme.reading_form(), "トウキョウト");
    assert_eq!(morpheme.part_of_speech_id(), 3);
    assert_eq!(
        morpheme.part_of_speech(),
        SudachiPartOfSpeech {
            level1: "名詞".into(),
            level2: "固有名詞".into(),
            level3: "地名".into(),
            level4: "一般".into(),
            conjugation_type: "*".into(),
            conjugation_form: "*".into(),
        }
    );
    assert_eq!(morpheme.dictionary_id(), 0);
    assert!(!morpheme.is_oov());

    let split = morpheme
        .split(SudachiSplitMode::A, None)
        .expect("split should succeed");
    assert_eq!(split.count(), 2);
    assert_eq!(
        split
            .morpheme_at(0)
            .expect("split morpheme 0 should exist")
            .surface(),
        "東京"
    );
    assert_eq!(
        split
            .morpheme_at(1)
            .expect("split morpheme 1 should exist")
            .surface(),
        "都"
    );
}

#[test]
fn word_info_and_user_dictionary_metadata_are_exposed() {
    let dict = test_dictionary(None);
    let tokenizer = dict.create_tokenizer(Some(SudachiSplitMode::C), None, None);

    let tokyo_pref = tokenizer
        .tokenize("東京府")
        .expect("tokenization should succeed")
        .morpheme_at(0)
        .expect("first morpheme should exist");
    let info = tokyo_pref.word_info();
    assert_eq!(info.surface, "東京府");
    assert_eq!(info.head_word_length, 9);
    assert_eq!(info.pos_id, 3);
    assert_eq!(info.dictionary_form_word_id, -1);
    assert_eq!(info.dictionary_form, "東京府");
    assert_eq!(info.normalized_form, "東京府");
    assert_eq!(info.reading_form, "トウキョウフ");
    assert_eq!(info.a_unit_split, vec![5, (1u32 << 28) + 1]);
    assert_eq!(info.b_unit_split, Vec::<u32>::new());
    assert_eq!(info.word_structure, vec![5, (1u32 << 28) + 1]);
    assert_eq!(info.synonym_group_ids, vec![1, 3]);

    let user_word = tokenizer
        .tokenize("ぴらる")
        .expect("user dictionary tokenization should succeed")
        .morpheme_at(0)
        .expect("first morpheme should exist");
    assert_eq!(user_word.word_id(), 1u32 << 28);
    assert_eq!(user_word.dictionary_id(), 1);

    let oov = tokenizer
        .tokenize("京")
        .expect("oov tokenization should succeed")
        .morpheme_at(0)
        .expect("first morpheme should exist");
    assert!(oov.is_oov());
    assert!(oov.dictionary_id() < 0);
}

#[test]
fn lookup_and_part_of_speech_queries_work() {
    let dict = test_dictionary(None);

    let pos = dict.part_of_speech(3).expect("POS id 3 should exist");
    assert_eq!(
        pos,
        SudachiPartOfSpeech {
            level1: "名詞".into(),
            level2: "固有名詞".into(),
            level3: "地名".into(),
            level4: "一般".into(),
            conjugation_type: "*".into(),
            conjugation_form: "*".into(),
        }
    );

    let looked_up = dict.lookup("ぴらる").expect("lookup should succeed");
    assert_eq!(looked_up.count(), 1);
    let morpheme = looked_up
        .morpheme_at(0)
        .expect("looked up morpheme should exist");
    assert_eq!(morpheme.surface(), "ぴらる");
    assert_eq!(morpheme.dictionary_id(), 1);
    assert_eq!(morpheme.word_id(), (1u32 << 28) + 0);
}

#[test]
fn lookup_uses_default_projection_and_preserves_boundaries() {
    let dict = test_dictionary(Some("reading"));

    let looked_up = dict.lookup("東京都").expect("lookup should succeed");
    assert_eq!(looked_up.count(), 1);
    let morpheme = looked_up
        .morpheme_at(0)
        .expect("looked up morpheme should exist");
    assert_eq!(morpheme.surface(), "トウキョウト");
    assert_eq!(morpheme.raw_surface(), "東京都");
    assert_eq!(morpheme.begin(), 0);
    assert_eq!(morpheme.end(), 3);

    let split = morpheme
        .split(SudachiSplitMode::A, None)
        .expect("lookup split should succeed");
    assert_eq!(split.count(), 2);
    assert_eq!(
        split
            .morpheme_at(0)
            .expect("split morpheme 0 should exist")
            .surface(),
        "トウキョウ"
    );
    assert_eq!(
        split
            .morpheme_at(0)
            .expect("split morpheme 0 should exist")
            .raw_surface(),
        "東京"
    );
    assert_eq!(
        split
            .morpheme_at(1)
            .expect("split morpheme 1 should exist")
            .surface(),
        "ト"
    );
}

#[test]
fn empty_and_error_paths_return_stable_results() {
    let dict = test_dictionary(None);
    let tokenizer = dict.create_tokenizer(Some(SudachiSplitMode::C), None, None);

    let empty_lookup = dict.lookup("存在しない").expect("lookup should succeed");
    assert!(empty_lookup.is_empty());
    assert_eq!(empty_lookup.count(), 0);
    assert_eq!(empty_lookup.internal_cost(), 0);
    assert!(dict.part_of_speech(u16::MAX).is_none());

    let list = tokenizer
        .tokenize("京都")
        .expect("tokenization should succeed");
    let err = match list.morpheme_at(1) {
        Ok(_) => panic!("out of range index should fail"),
        Err(err) => err,
    };
    assert!(matches!(err, SudachiError::InvalidArgument(_)));

    let morpheme = list.morpheme_at(0).expect("first morpheme should exist");
    let empty_split = morpheme
        .split(SudachiSplitMode::C, Some(false))
        .expect("mode C split should succeed");
    assert!(empty_split.is_empty());
    assert_eq!(empty_split.count(), 0);
    assert_eq!(empty_split.internal_cost(), 0);
}

#[test]
fn pos_matcher_supports_matching_and_set_operations() {
    let dict = test_dictionary(None);
    let noun_matcher = dict
        .make_pos_matcher(vec![SudachiPartialPartOfSpeech {
            level1: Some("名詞".into()),
            ..Default::default()
        }])
        .expect("noun matcher should be created");
    let terminal_form_matcher = dict
        .make_pos_matcher(vec![SudachiPartialPartOfSpeech {
            conjugation_form: Some("終止形-一般".into()),
            ..Default::default()
        }])
        .expect("terminal matcher should be created");

    let tokenizer = dict.create_tokenizer(Some(SudachiSplitMode::A), None, None);
    let list = tokenizer
        .tokenize("東京に行く")
        .expect("tokenization should succeed");
    let tokyo = list.morpheme_at(0).expect("first morpheme should exist");
    let iku = list.morpheme_at(2).expect("third morpheme should exist");

    assert!(noun_matcher.matches(&tokyo).expect("matcher should work"));
    assert!(!noun_matcher.matches(&iku).expect("matcher should work"));

    let union = noun_matcher
        .union(&terminal_form_matcher)
        .expect("union should succeed");
    assert!(union.count() >= noun_matcher.count());

    let intersection = noun_matcher
        .intersection(&terminal_form_matcher)
        .expect("intersection should succeed");
    assert_eq!(intersection.count(), 0);

    let subtraction = union
        .subtract(&noun_matcher)
        .expect("subtract should succeed");
    assert!(subtraction.count() <= union.count());

    let inverted = noun_matcher.inverted();
    assert!(inverted.count() > 0);
    assert!(!inverted.matches(&tokyo).expect("matcher should work"));
}

#[test]
fn pos_matcher_rejects_invalid_patterns_and_cross_dictionary_objects() {
    let dict = test_dictionary(None);
    let other_dict = test_dictionary(None);

    let err = match dict.make_pos_matcher(vec![SudachiPartialPartOfSpeech {
        level1: Some("存在しない品詞".into()),
        ..Default::default()
    }]) {
        Ok(_) => panic!("unmatched pattern should fail"),
        Err(err) => err,
    };
    assert!(matches!(err, SudachiError::InvalidArgument(_)));

    let noun_matcher = dict
        .make_pos_matcher(vec![SudachiPartialPartOfSpeech {
            level1: Some("名詞".into()),
            ..Default::default()
        }])
        .expect("noun matcher should be created");
    let other_noun_matcher = other_dict
        .make_pos_matcher(vec![SudachiPartialPartOfSpeech {
            level1: Some("名詞".into()),
            ..Default::default()
        }])
        .expect("other noun matcher should be created");
    let other_tokenizer = other_dict.create_tokenizer(Some(SudachiSplitMode::C), None, None);
    let other_morpheme = other_tokenizer
        .tokenize("東京")
        .expect("tokenization should succeed")
        .morpheme_at(0)
        .expect("first morpheme should exist");

    let err = match noun_matcher.matches(&other_morpheme) {
        Ok(_) => panic!("cross-dictionary matcher usage should fail"),
        Err(err) => err,
    };
    assert!(matches!(err, SudachiError::InvalidArgument(_)));

    let err = match noun_matcher.union(&other_noun_matcher) {
        Ok(_) => panic!("cross-dictionary matcher union should fail"),
        Err(err) => err,
    };
    assert!(matches!(err, SudachiError::InvalidArgument(_)));
}

#[test]
fn fields_control_split_capability_without_breaking_basic_pos_access() {
    let dict = test_dictionary(None);
    let tokenizer = dict.create_tokenizer(
        Some(SudachiSplitMode::C),
        Some(vec![SudachiInfoField::PartOfSpeech]),
        None,
    );

    let list = tokenizer
        .tokenize("東京都")
        .expect("subset tokenization should succeed");
    let morpheme = list.morpheme_at(0).expect("first morpheme should exist");
    assert_eq!(morpheme.part_of_speech_id(), 3);
    assert_eq!(morpheme.word_info().surface, "東京都");

    let err = match morpheme.split(SudachiSplitMode::A, None) {
        Ok(_) => panic!("split should require split_a data"),
        Err(err) => err,
    };
    assert!(matches!(err, SudachiError::InvalidArgument(_)));
}
