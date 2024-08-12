use nu_cli::add_cli_context;
use nu_cmd_extra::*;
use nu_cmd_lang::*;
use nu_cmd_plugin::*;
use nu_command::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

// We maintain our own default context so that we can control the commands supported,
// and prevent name clashes. This function should be very similar to the same one in nu_command.
pub fn create_default_context() -> EngineState {
    let engine_state = EngineState::new();

    // If there are commands that have the same name as default declarations,
    // they have to be registered before the main declarations. This helps to make
    // them only accessible if the correct input value category is used with the
    // declaration.
    // These commands typically all start with dfr so we're safe to blindly add them all.
    // let engine_state = add_dataframe_context(engine_state);
    let engine_state = add_extra_command_context(engine_state);
    let engine_state = add_cli_context(engine_state);
    let mut engine_state = nu_explore::add_explore_context(engine_state);

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // Core, from nu_cmd_lang
        bind_command! {
            Alias,
            Break,
            Collect,
            Const,
            Continue,
            Def,
            Describe,
            Do,
            Echo,
            ErrorMake,
            ExportAlias,
            ExportCommand,
            ExportDef,
            ExportExtern,
            ExportUse,
            ExportModule,
            Extern,
            For,
            Hide,
            HideEnv,
            If,
            Ignore,
            Overlay,
            OverlayUse,
            OverlayList,
            OverlayNew,
            OverlayHide,
            Let,
            Loop,
            Match,
            Module,
            Mut,
            Version,
            While,
        };

        // Charts, from nu_command
        bind_command! {
            Histogram
        }

        // Help, from nu_command
        bind_command!(
            HelpAliases,
            HelpCommands,
            HelpModules,
            HelpExterns,
            HelpOperators,
        );

        // Filters, from nu_command
        bind_command! {
            All,
            Any,
            Append,
            Columns,
            Compact,
            Default,
            Drop,
            DropColumn,
            DropNth,
            Each,
            Enumerate,
            Every,
            Filter,
            Find,
            First,
            Flatten,
            Get,
            Group,
            GroupBy,
            Headers,
            Insert,
            Items,
            Join,
            SplitBy,
            Take,
            Merge,
            Move,
            TakeWhile,
            TakeUntil,
            Last,
            Length,
            Lines,
            ParEach,
            Prepend,
            Range,
            Reduce,
            Reject,
            Rename,
            Reverse,
            Select,
            Shuffle,
            Skip,
            SkipUntil,
            SkipWhile,
            Sort,
            SortBy,
            SplitList,
            Transpose,
            Uniq,
            UniqBy,
            Upsert,
            Update,
            Values,
            Where,
            Window,
            Wrap,
            Zip,
        };

        // Misc, from nu_command
        bind_command! {
            Source,
            // Tutor, TODO(chvck): useful but we need to think about how this interacts with our tutorial
        };

        // Path, from nu_command
        bind_command! {
            Path,
            PathBasename,
            PathDirname,
            PathExists,
            PathExpand,
            PathJoin,
            PathParse,
            PathRelativeTo,
            PathSplit,
            PathType,
        };

        // System, from nu_command
        bind_command! {
            Complete,
            External,
            NuCheck,
            Sys,
        };

        #[cfg(unix)]
        bind_command! { Exec }

        #[cfg(windows)]
        bind_command! { RegistryQuery }

        #[cfg(any(
            target_os = "android",
            target_os = "linux",
            target_os = "macos",
            target_os = "windows"
        ))]
        bind_command! { Ps };

        bind_command! { Which };

        // Strings, from nu_command
        bind_command! {
            Char,
            Decode,
            Encode,
            DecodeBase64,
            EncodeBase64,
            DetectColumns,
            Parse,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            SplitWords,
            Str,
            StrStats,
            StrCapitalize,
            StrContains,
            StrDistance,
            StrDowncase,
            StrEndswith,
            StrJoin,
            StrReplace,
            StrIndexOf,
            StrLength,
            StrReverse,
            StrStartsWith,
            StrSubstring,
            StrTrim,
            StrUpcase
        };

        // FileSystem, from nu_command
        bind_command! {
            Cd,
            UCp,
            Ls,
            UMkdir,
            UMv,
            Open,
            Rm,
            Save,
            Touch,
            Glob,
            Watch,
        };

        // Platform, from nu_command
        bind_command! {
            Ansi,
            AnsiLink,
            AnsiStrip,
            Clear,
            Du,
            Input,
            InputList,
            InputListen,
            IsTerminal,
            Kill,
            Sleep,
            TermSize,
            Whoami
        };

        // Date, from nu_command
        bind_command! {
            Date,
            DateHumanize,
            DateListTimezones,
            DateNow,
            DateToRecord,
            DateToTable,
            DateToTimezone,
        };

        // Shells, from nu_command
        bind_command! {
            Exit,
        };

        // Formats, from nu_command
        bind_command! {
            From,
            FromCsv,
            FromJson,
            FromNuon,
            FromOds,
            FromSsv,
            FromToml,
            FromTsv,
            FromXlsx,
            FromXml,
            FromYaml,
            FromYml,
            To,
            ToCsv,
            ToJson,
            ToMd,
            ToNuon,
            ToText,
            ToToml,
            ToTsv,
            Touch,
            Use,
            Upsert,
            Where,
            ToXml,
            ToYaml,
        };

        // Viewers, from nu_command
        bind_command! {
            Griddle,
            Table,
        };

        // Conversions, from nu_command
        bind_command! {
            Into,
            IntoBool,
            IntoBinary,
            IntoCellPath,
            IntoDatetime,
            IntoDuration,
            IntoFloat,
            IntoFilesize,
            IntoGlob,
            IntoInt,
            IntoRecord,
            IntoString,
        };

        // Env, from nu_command
        bind_command! {
            ExportEnv,
            LoadEnv,
            SourceEnv,
            WithEnv,
            ConfigNu,
            ConfigEnv,
            ConfigMeta,
            ConfigReset,
        };

        // Math, from nu_command
        bind_command! {
            Math,
            MathAbs,
            MathAvg,
            MathCeil,
            MathFloor,
            MathMax,
            MathMedian,
            MathMin,
            MathMode,
            MathProduct,
            MathRound,
            MathSqrt,
            MathStddev,
            MathSum,
            MathVariance,
            MathLog,
        };

        // Network, from nu_command
        bind_command! {
            Http,
            HttpDelete,
            HttpGet,
            HttpHead,
            HttpPatch,
            HttpPost,
            HttpPut,
            HttpOptions,
            Url,
            UrlBuildQuery,
            UrlEncode,
            UrlJoin,
            UrlParse,
            Port,
        }

        // Random, from nu_command
        bind_command! {
            Random,
            RandomBool,
            RandomChars,
            RandomFloat,
            RandomDice,
            RandomInt,
            RandomUuid,
        };

        // Generators, from nu_command
        bind_command! {
            Cal,
            Seq,
            SeqDate,
            SeqChar,
        };

        // Hash, from nu_command
        bind_command! {
            Hash,
            HashMd5::default(),
            HashSha256::default(),
        };

        // Experimental
        bind_command! {
            IsAdmin,
        };

        // Bytes, from nu_command {
        bind_command! {
            Bytes,
            BytesAdd,
            BytesAt,
            BytesBuild,
            BytesCollect,
            BytesEndsWith,
            BytesIndexOf,
            BytesLen,
            BytesRemove,
            BytesReplace,
            BytesReverse,
            BytesStartsWith
        }

        //Debug, from nu_command
        bind_command! {
            Ast,
            Debug,
            DebugInfo,
            DebugProfile,
            Explain,
            Inspect,
            Metadata,
            MetadataSet,
            TimeIt,
            View,
            ViewFiles,
            ViewSource,
            ViewSpan,
        }

        //Plugin, from nu_command
        bind_command! {
            PluginCommand,
            PluginAdd,
            PluginList,
            PluginRm,
            PluginStop,
            PluginUse,
        }

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating default context: {err:?}");
    }

    // Cache the table decl id so we don't have to look it up later
    let table_decl_id = engine_state.find_decl("table".as_bytes(), &[]);
    engine_state.table_decl_id = table_decl_id;

    engine_state
}
