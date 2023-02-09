use nu_command::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

// We maintain our own default context so that we can control the commands supported,
// and prevent name clashes. This function should be very similar to the same one in nu_command.
pub fn create_default_context() -> EngineState {
    let mut engine_state = EngineState::new();

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // If there are commands that have the same name as default declarations,
        // they have to be registered before the main declarations. This helps to make
        // them only accessible if the correct input value category is used with the
        // declaration.
        // These commands typically all start with dfr so we're safe to blindly add them all.
        add_dataframe_decls(&mut working_set);

        // Core
        bind_command! {
            Alias,
            Ast,
            Break,
            Commandline,
            Continue,
            Debug,
            Def,
            DefEnv,
            Describe,
            Do,
            Echo,
            ErrorMake,
            ExportAlias,
            ExportCommand,
            ExportDef,
            ExportDefEnv,
            ExportExtern,
            ExportUse,
            Extern,
            For,
            // Help,
            HelpAliases,
            HelpCommands,
            HelpModules,
            HelpOperators,
            HelpOperators,
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
            Metadata,
            Module,
            Mut,
            Return,
            Try,
            Use,
            // Version,
            While,
        };

        // Charts
        bind_command! {
            Histogram
        }

        // Filters
        bind_command! {
            All,
            Any,
            Append,
            Collect,
            Columns,
            Compact,
            Default,
            Drop,
            DropColumn,
            DropNth,
            Each,
            EachWhile,
            Empty,
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
            Roll,
            RollDown,
            RollUp,
            RollLeft,
            RollRight,
            Rotate,
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
            UpdateCells,
            Where,
            Window,
            Wrap,
            Zip,
        };

        // Misc
        bind_command! {
            History,
            // Tutor, TODO(chvck): useful but we need to think about how this interacts with our tutorial
            HistorySession,
        };

        // Path
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

        // System
        bind_command! {
            Benchmark,
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

        // Strings
        bind_command! {
            Char,
            Decode,
            Encode,
            DecodeBase64,
            EncodeBase64,
            DetectColumns,
            Format,
            FileSize,
            Parse,
            Size,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            SplitWords,
            Str,
            StrCamelCase,
            StrCapitalize,
            StrCollect,
            StrContains,
            StrDistance,
            StrDowncase,
            StrEndswith,
            StrJoin,
            StrReplace,
            StrIndexOf,
            StrKebabCase,
            StrLength,
            StrLpad,
            StrPascalCase,
            StrReverse,
            StrRpad,
            StrScreamingSnakeCase,
            StrSnakeCase,
            StrStartsWith,
            StrSubstring,
            StrTrim,
            StrTitleCase,
            StrUpcase
        };

        // Bits
        bind_command! {
            Bits,
            BitsAnd,
            BitsNot,
            BitsOr,
            BitsXor,
            BitsRotateLeft,
            BitsRotateRight,
            BitsShiftLeft,
            BitsShiftRight,
        }

        // Bytes
        bind_command! {
            Bytes,
            BytesLen,
            BytesStartsWith,
            BytesEndsWith,
            BytesReverse,
            BytesReplace,
            BytesAdd,
            BytesAt,
            BytesIndexOf,
            BytesCollect,
            BytesRemove,
            BytesBuild,
        }

        // FileSystem
        bind_command! {
            Cd,
            Cp,
            Ls,
            Mkdir,
            Mv,
            Open,
            Rm,
            Save,
            Touch,
            Glob,
            Watch,
        };

        // Platform
        bind_command! {
            Ansi,
            AnsiGradient,
            AnsiStrip,
            Clear,
            Du,
            KeybindingsDefault,
            Input,
            KeybindingsListen,
            Keybindings,
            Kill,
            KeybindingsList,
            Sleep,
            TermSize,
        };

        // Date
        bind_command! {
            Date,
            DateFormat,
            DateHumanize,
            DateListTimezones,
            DateNow,
            DateToRecord,
            DateToTable,
            DateToTimezone,
        };

        // Shells
        bind_command! {
            Enter,
            Exit,
            GotoShell,
            NextShell,
            PrevShell,
            Shells,
        };

        // Formats
        bind_command! {
            From,
            FromCsv,
            FromEml,
            FromIcs,
            FromIni,
            FromJson,
            FromNuon,
            FromOds,
            FromSsv,
            FromToml,
            FromTsv,
            FromUrl,
            FromVcf,
            FromXlsx,
            FromXml,
            FromYaml,
            FromYml,
            To,
            ToCsv,
            ToHtml,
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

        // Viewers
        bind_command! {
            Griddle,
            Table,
            Explore,
        };

        // Conversions
        bind_command! {
            Fmt,
            Into,
            IntoBool,
            IntoBinary,
            IntoDatetime,
            IntoDecimal,
            IntoDuration,
            IntoFilesize,
            IntoInt,
            IntoRecord,
            IntoString,
        };

        // Env
        bind_command! {
            Env,
            ExportEnv,
            LetEnv,
            LoadEnv,
            SourceEnv,
            WithEnv,
            ConfigNu,
            ConfigEnv,
            ConfigMeta,
            ConfigReset,
        };

        // Math
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
            MathSin,
            MathCos,
            MathTan,
            MathSinH,
            MathCosH,
            MathTanH,
            MathArcSin,
            MathArcCos,
            MathArcTan,
            MathArcSinH,
            MathArcCosH,
            MathArcTanH,
            MathPi,
            MathTau,
            MathEuler,
            MathLn,
            MathLog,
        };

        // Network
        bind_command! {
            Http,
            HttpGet,
            HttpPost,
            Url,
            UrlBuildQuery,
            UrlEncode,
            UrlJoin,
            UrlParse,
            Port,
        }

        // Random
        bind_command! {
            Random,
            RandomBool,
            RandomChars,
            RandomDecimal,
            RandomDice,
            RandomInteger,
            RandomUuid,
        };

        // Generators
        bind_command! {
            Cal,
            Seq,
            SeqDate,
            SeqChar,
        };

        // Hash
        bind_command! {
            Hash,
            HashMd5::default(),
            HashSha256::default(),
        };

        // Experimental
        bind_command! {
            ViewSource,
            IsAdmin,
        };

        bind_command!(Register);

        working_set.render()
    };

    let _ = engine_state.merge_delta(delta);

    engine_state
}
