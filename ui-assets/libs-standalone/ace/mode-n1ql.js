(function() {

  //
  // some globals used by both the highlighter and the autocompleter
  //

  var keywords = (
      "ADVISE|ALL|ALTER|ANALYZE|AND|ANY|ARRAY|AS|ASC|BEGIN|BETWEEN|BINARY|BOOLEAN|BREAK|BUCKET|BUILD|BY|CALL|CASE|CAST|" +
      "CLUSTER|COLLATE|COLLECTION|COMMIT|CONNECT|CONTINUE|CORRELATED|CORRELATE|COVER|CREATE|CURRENT|DATABASE|DATASET|" +
      "DATASTORE|DECLARE|DECREMENT|DELETE|DERIVED|DESC|DESCRIBE|DISTINCT|DO|DROP|EACH|ELEMENT|ELSE|END|EVERY|EXCEPT|" +
      "EXCLUDE|EXECUTE|EXISTS|EXPLAIN|FETCH|FIRST|FLATTEN|FOLLOWING|FOR|FORCE|FROM|FTS|FUNCTION|GOLANG|GRANT|GROUP|" +
      "GROUPS|GSI|HASH|HAVING|IF|IGNORE|ILIKE|IN|INCLUDE|INCREMENT|INDEX|INFER|INLINE|INNER|INSERT|INTERSECT|INTO|IS|" +
      "JAVASCRIPT|JOIN|KEY|KEYS|KEYSPACE|KNOWN|LANGUAGE|LAST|LEFT|LET|LETTING|LIKE|LIMIT|LSM|MAP|MAPPING|MATCHED|" +
      "MATERIALIZED|MERGE|MINUS|MISSING|NAMESPACE|NAMESPACE_ID|NEST|NL|NO|NOT|NOT_A_TOKEN|NTH_VALUE|NULL|NULLS|NUMBER|" +
      "OBJECT|OFFSET|ON|OPTION|OR|ORDER|OTHERS|OUTER|OVER|PARSE|PARTITION|PASSWORD|PATH|POOL|PRECEDING|PREPARE|PRIMARY|" +
      "PRIVATE|PRIVILEGE|PROBE|PROCEDURE|PUBLIC|RANGE|RAW|REALM|REDUCE|RENAME|RESPECT|RETURN|RETURNING|REVOKE|RIGHT|ROLE|" +
      "ROLLBACK|ROW|ROWS|SATISFIES|SCHEMA|SELECT|SELF|SEMI|SET|SHOW|SOME|START|STATISTICS|STRING|SYSTEM|THEN|TIES|TO|" +
      "TRANSACTION|TRIGGER|TRUNCATE|UNBOUNDED|UNDER|UNION|UNIQUE|UNKNOWN|UNNEST|UNSET|UPDATE|UPSERT|USE|USER|USING|" +
      "VALIDATE|VALUE|VALUED|VALUES|VIA|VIEW|WHEN|WHERE|WHILE|WITH|WITHIN|WORK|XOR"
  );
  var keywords_array = keywords.split('|');

  var sysCatalogs = (
      "system:active_requests|system:applicable_roles|system:completed_requests|system:datastores|system:dual|" +
      "system:functions|system:functions_cache|system:indexes|system:keyspaces|system:my_user_info|system:namespaces|" +
      "system:nodes|system:prepareds|system:user_info"
  );
  var sysCatalogs_array = sysCatalogs.split('|');

  var roles = (
      "ADMIN|RO_ADMIN|CLUSTER_ADMIN|BUCKET_ADMIN|BUCKET_ADMIN|BUCKET_ADMIN|BUCKET_ADMIN|BUCKET_ADMIN|BUCKET_SASL|" +
      "BUCKET_SASL|BUCKET_SASL|BUCKET_SASL|BUCKET_SASL|VIEWS_ADMIN|VIEWS_ADMIN|VIEWS_ADMIN|VIEWS_ADMIN|VIEWS_ADMIN|" +
      "REPLICATION_ADMIN|DATA_READER|DATA_READER|DATA_READER|DATA_READER|DATA_READER|DATA_READER_WRITER|" +
      "DATA_READER_WRITER|DATA_READER_WRITER|DATA_READER_WRITER|DATA_READER_WRITER|DATA_DCP_READER|DATA_DCP_READER|" +
      "DATA_DCP_READER|DATA_DCP_READER|DATA_DCP_READER|DATA_BACKUP|DATA_BACKUP|DATA_BACKUP|DATA_BACKUP|DATA_BACKUP|" +
      "DATA_MONITORING|DATA_MONITORING|DATA_MONITORING|DATA_MONITORING|DATA_MONITORING|FTS_ADMIN|FTS_ADMIN|FTS_ADMIN|" +
      "FTS_ADMIN|FTS_ADMIN|FTS_SEARCHER|FTS_SEARCHER|FTS_SEARCHER|FTS_SEARCHER|FTS_SEARCHER|QUERY_SELECT|QUERY_SELECT|" +
      "QUERY_SELECT|QUERY_SELECT|QUERY_SELECT|QUERY_UPDATE|QUERY_UPDATE|QUERY_UPDATE|QUERY_UPDATE|QUERY_UPDATE|" +
      "QUERY_INSERT|QUERY_INSERT|QUERY_INSERT|QUERY_INSERT|QUERY_INSERT|QUERY_DELETE|QUERY_DELETE|QUERY_DELETE|" +
      "QUERY_DELETE|QUERY_DELETE|QUERY_MANAGE_INDEX|QUERY_MANAGE_INDEX|QUERY_MANAGE_INDEX|QUERY_MANAGE_INDEX|" +
      "QUERY_MANAGE_INDEX|QUERY_SYSTEM_CATALOG|QUERY_EXTERNAL_ACCESS"
  );
  var roles_array = roles.split('|');

  var builtinConstants = (
      "TRUE|FALSE|INDEXES|KEYSPACES"
  );
  var builtinConstants_array = builtinConstants.split('|');

  // this list of functions should be updated w.r.t. https://github.com/couchbase/query/blob/master/expression/func_registry.go
  var Arithmetic = "ADD|DIV|IDIV|IMOD|MOD|MULT|NEG|SUB";
  var Comparison = "BETWEEN|EQ|IS_KNOWN|IS_MISSING|IS_NOT_KNOWN|IS_NOT_MISSING|IS_NOT_NULL|IS_NOT_VALUED|IS_NOT_UNKNOWN|" +
  		"IS_NULL|IS_VALUED|ISKNOWN|ISMISSING|ISNOTKNOWN|ISNOTMISSING|ISNOTNULL|ISNOTUNKNOWN|ISNOTVALUED|ISNULL|ISUNKNOWN|" +
  		"ISVALUED|LE|LIKE|LT|LIKE_PREFIX|LIKE_STOP|LIKE_SUFFIX|REGEXP_PREFIX|REGEXP_STOP|REGEXP_SUFFIX";
  var Concat = "CONCAT|CONCAT2";
  var Costruction = "ARRAY";
  var Navigation = "ELEMENT|FIELD|SLICE";
  var Curl = "CURL";
  var Date = "CLOCK_LOCAL|CLOCK_MILLIS|CLOCK_STR|CLOCK_TZ|CLOCK_UTC|DATE_ADD_MILLIS|DATE_ADD_STR|DATE_DIFF_MILLIS|" +
  		"DATE_DIFF_STR|DATE_DIFF_ABS_STR|DATE_DIFF_ABS_MILLIS|DATE_FORMAT_STR|DATE_PART_MILLIS|DATE_PART_STR|" +
  		"DATE_RANGE_MILLIS|DATE_RANGE_STR|DATE_TRUNC_MILLIS|DATE_TRUNC_STR|DURATION_TO_STR|MILLIS|MILLIS_TO_LOCAL|" +
  		"MILLIS_TO_STR|MILLIS_TO_TZ|MILLIS_TO_UTC|MILLIS_TO_ZONE_NAME|NOW_LOCAL|NOW_MILLIS|NOW_STR|NOW_TZ|NOW_UTC|" +
  		"STR_TO_DURATION|STR_TO_MILLIS|STR_TO_TZ|STR_TO_UTC|STR_TO_ZONE_NAME|WEEKDAY_MILLIS|WEEKDAY_STR";
  var String = "CONTAINS|INITCAP|LENGTH|LOWER|LTRIM|POSITION|POS|POSITION0|POS0|POSITION1|POS1|REPEAT|REPLACE|REVERSE|" +
  		"RTRIM|SPLIT|SUBSTR|SUBSTR0|SUBSTR1|SUFFIXES|TITLE|TRIM|UPPER";
  var Regular_expressions = "CONTAINS_REGEX|CONTAINS_REGEXP|REGEX_CONTAINS|REGEX_LIKE|REGEX_POSITION0|REGEX_POS0|" +
  		"REGEXP_POSITION0|REGEXP_POS0|REGEX_POSITION1|REGEX_POS1|REGEXP_POSITION1|REGEXP_POS1|REGEX_POSITION|REGEX_POS|" +
  		"REGEX_REPLACE|REGEXP_CONTAINS|REGEXP_LIKE|REGEXP_POSITION|REGEXP_POS|REGEXP_REPLACE|REGEXP_MATCHES|REGEXP_SPLIT";
  var Numeric = "ABS|ACOS|ASIN|ATAN|ATAN2|CEIL|COS|DEG|DEGREES|E|EXP|LN|LOG|FLOOR|INF|NAN|NEGINF|NEG_INF|PI|POSINF|" +
  		"POS_INF|POWER|RAD|RADIANS|RANDOM|ROUND|SIGN|SIN|SQRT|TAN|TRUNC";
  var Bitwise = "BITAND|BITOR|BITXOR|BITNOT|BITSHIFT|BITSET|BITCLEAR|BITTEST|ISBITSET";
  var Array = "ARRAY_ADD|ARRAY_APPEND|ARRAY_AVG|ARRAY_CONCAT|ARRAY_CONTAINS|ARRAY_COUNT|ARRAY_DISTINCT|ARRAY_FLATTEN|" +
  		"ARRAY_IFNULL|ARRAY_INSERT|ARRAY_INTERSECT|ARRAY_LENGTH|ARRAY_MAX|ARRAY_MIN|ARRAY_POSITION|ARRAY_POS|ARRAY_PREPEND|" +
  		"ARRAY_PUT|ARRAY_RANGE|ARRAY_REMOVE|ARRAY_REPEAT|ARRAY_REPLACE|ARRAY_REVERSE|ARRAY_SORT|ARRAY_STAR|ARRAY_SUM|" +
  		"ARRAY_SYMDIFF|ARRAY_SYMDIFF1|ARRAY_SYMDIFFN|ARRAY_UNION|ARRAY_SWAP|ARRAY_MOVE|ARRAY_EXCEPT|ARRAY_BINARY_SEARCH";
  var Object = "OBJECT_ADD|OBJECT_CONCAT|OBJECT_INNER_PAIRS|OBJECT_INNERPAIRS|OBJECT_INNER_VALUES|OBJECT_INNERVALUES|" +
  		"OBJECT_LENGTH|OBJECT_NAMES|OBJECT_OUTER_PAIRS|OBJECT_OUTERPAIRS|OBJECT_OUTER_VALUES|OBJECT_OUTERVALUES|" +
  		"OBJECT_PAIRS|OBJECT_PUT|OBJECT_REMOVE|OBJECT_RENAME|OBJECT_REPLACE|OBJECT_UNWRAP|OBJECT_VALUES";
  var Json = "DECODE_JSON|ENCODE_JSON|ENCODED_SIZE|JSON_DECODE|JSON_ENCODE|PAIRS|POLY_LENGTH";
  var Base64 = "BASE64|BASE64_DECODE|BASE64_ENCODE|DECODE_BASE64|ENCODE_BASE64";
  var Comparison2 = "GREATEST|LEAST|SUCCESSOR";
  var Token = "CONTAINS_TOKEN|CONTAINS_TOKEN_LIKE|CONTAINS_TOKEN_REGEX|CONTAINS_TOKEN_REGEXP|HAS_TOKEN|TOKENS";
  var Conditional_for_unknowns = "IF_MISSING|IF_MISSING_OR_NULL|IF_NULL|MISSING_IF|NULL_IF|IFMISSING|IFMISSINGORNULL|" +
  		"IFNULL|MISSINGIF|NULLIF|COALESCE|NVL|NVL2";
  var Conditional_for_numbers = "IF_INF|IF_NAN|IF_NAN_OR_INF|NAN_IF|NEGINF_IF|POSINF_IF|IFINF|IFNAN|IFNANORINF|NANIF|" +
  		"NEGINFIF|POSINFIF";
  var Meta = "META|MIN_VERSION|SELF|UUID|VERSION|CURRENT_USERS|DS_VERSION";
  var Distributed = "NODE_NAME";
  var Type_checking = "IS_ARRAY|IS_ATOM|IS_BIN|IS_BINARY|IS_BOOL|IS_BOOLEAN|IS_NUM|IS_NUMBER|IS_OBJ|IS_OBJECT|IS_STR|" +
  		"IS_STRING|ISARRAY|ISATOM|ISBIN|ISBINARY|ISBOOL|ISBOOLEAN|ISNUM|ISNUMBER|ISOBJ|ISOBJECT|ISSTR|ISSTRING|TYPE|" +
  		"TYPE_NAME|TYPENAME";
  var Type_conversion = "TO_ARRAY|TO_ATOM|TO_BOOL|TO_BOOLEAN|TO_NUM|TO_NUMBER|TO_OBJ|TO_OBJECT|TO_STR|TO_STRING|TOARRAY|" +
  		"TOATOM|TOBOOL|TOBOOLEAN|TONUM|TONUMBER|TOOBJ|TOOBJECT|TOSTR|TOSTRING|DECODE";
  var Unnest = "UNNEST_POSITION|UNNEST_POS";
  var Index_Advisor = "ADVISOR";

  // plus aggregates from https://github.com/couchbase/query/blob/master/algebra/agg_registry.go
  var Aggregates = "ARRAY_AGG|AVG|COUNT|COUNTN|MAX|MEAN|MEDIAN|MIN|STDDEV|STDDEV_POP|STDDEV_SAMP|SUM|VARIANCE|VAR_POP|" +
  		"VARIANCE_POP|VAR_SAMP|VARIANCE_SAMP|ROW_NUMBER|RANK|DENSE_RANK|PERCENT_RANK|CUME_DIST|RATIO_TO_REPORT|NTILE|" +
  		"FIRST_VALUE|LAST_VALUE|NTH_VALUE|LAG|LEAD";

  var builtinFunctions = (
      Arithmetic + "|" + Comparison + "|" + Concat + "|" + Costruction + "|" +
      Navigation + "|" + Curl + "|" + Date + "|" + String + "|" + Regular_expressions + "|" + Numeric + "|" +
      Bitwise + "|" + Array + "|" + Object + "|" + Json + "|" + Base64 + "|" + Comparison2 + "|" + Token + "|" +
      Conditional_for_unknowns + "|" + Conditional_for_numbers + "|" + Meta + "|" + Distributed + "|" +
      Type_checking + "|" + Type_conversion + "|" + Unnest + "|" + Index_Advisor + "|" + Aggregates
  );
  var builtinFunctions_array = builtinFunctions.split('|');

  //
  // put all categories of keywords in one data structure we can traverse
  //

  var terms = [
    {name:"keyword", tokens: keywords_array},
    {name:"built-in", tokens: builtinConstants_array},
    {name:"function", tokens: builtinFunctions_array},
    {name:"role", tokens: roles_array},
    {name:"system-catalog", tokens: sysCatalogs_array}
    ];

  //
  // language tokens
  //

  define("ace/mode/n1ql_highlight_rules",["require","exports","module","ace/lib/oop","ace/mode/text_highlight_rules"],
      function(require, exports, module) {
    "use strict";

    var oop = require("../lib/oop");
    var TextHighlightRules = require("./text_highlight_rules").TextHighlightRules;

    var N1qlHighlightRules = function() {

      var keywordMapper = this.createKeywordMapper({
        "support.function": builtinFunctions,
        "keyword": keywords,
        "constant.language": builtinConstants,
        "storage.type": roles
      }, "identifier", true);

      this.$rules = {
          "start" : [ {
            token : "comment",
            start : "/\\*",
            end : "\\*/"
          }, {
            token : "constant.numeric",   // " string, make blue like numbers
            regex : '".*?"'
          }, {
            token : "constant.numeric",   // ' string, make blue like numbers
            regex : "'.*?'"
          }, {
            token : "identifier",         // ` quoted identifier, make like identifiers
            regex : "[`](([`][`])|[^`])+[`]"
          }, {
            token : "constant.numeric",   // float
            regex : "[+-]?\\d+(?:(?:\\.\\d*)?(?:[eE][+-]?\\d+)?)?\\b"
          }, {
            token : keywordMapper,
            regex : "[a-zA-Z_$][a-zA-Z0-9_$]*\\b"
          }, {
            token : "keyword.operator",
            regex : "\\+|\\-|\\/|\\/\\/|%|<@>|@>|<@|&|\\^|~|<|>|<=|=>|==|!=|<>|="
          }, {
            token : "paren.lparen",
            regex : "[\\(]"
          }, {
            token : "paren.rparen",
            regex : "[\\)]"
          }, {
            token : "text",
            regex : "\\s+"
          } ]
      };
      this.normalizeRules();
    };

    oop.inherits(N1qlHighlightRules, TextHighlightRules);

    exports.N1qlHighlightRules = N1qlHighlightRules;
  });


  /*
   * Define the N1QL mode
   */

  define("ace/mode/n1ql_completions",["require","exports","module","ace/token_iterator"], function(require, exports, module) {
    "use strict";

    var TokenIterator = require("../token_iterator").TokenIterator;


    function is(token, type) {
      return token.type.lastIndexOf(type + ".xml") > -1;
    }

    function findTagName(session, pos) {
      var iterator = new TokenIterator(session, pos.row, pos.column);
      var token = iterator.getCurrentToken();
      while (token && !is(token, "tag-name")){
        token = iterator.stepBackward();
      }
      if (token)
        return token.value;
    }

    var N1qlCompletions = function() {
    };

    (function() {

      this.getCompletions = function(state, session, pos, prefix) {
        var token = session.getTokenAt(pos.row, pos.column);

        // return anything matching from the terms structure

        var results = [];
        var prefix_upper = prefix.toLocaleUpperCase();

        for (var i=0; i<terms.length; i++)
          for (var t=0; t<terms[i].tokens.length; t++)
            if (_.startsWith(terms[i].tokens[t].toLocaleUpperCase(),prefix_upper))
              results.push({value: terms[i].tokens[t], meta: terms[i].name, score: 1});

        return results;
      };


    }).call(N1qlCompletions.prototype);

    exports.N1qlCompletions = N1qlCompletions;
  });

  define("ace/mode/n1ql",["require","exports","module","ace/lib/oop","ace/mode/text","ace/mode/n1ql_highlight_rules",
    "ace/mode/query-formatter"],
      function(require, exports, module) {
    "use strict";

    var oop = require("../lib/oop");
    var TextMode = require("./text").Mode;
    var N1qlHighlightRules = require("./n1ql_highlight_rules").N1qlHighlightRules;
    var N1qlCompletions = require("./n1ql_completions").N1qlCompletions;

    //////////////////////////////////////////////////////////////////////////////////////
    // build a N1QL formatter from the more generic formatter package
    //
    // it needs to know keywords or function names (to be upper cased)
    //////////////////////////////////////////////////////////////////////////////////////

    // certain keywords will get formatted onto their own line, some with indenting
    var kw_regex_str = '\\b(?:' + sysCatalogs + ')|\\b(' + keywords + '|' + roles + '|' + builtinConstants + ')\\b';
    var function_regex_str = '\\b(' + builtinFunctions + ')\\s*\\(';

    var formatter = require("ace/mode/query-formatter").create(kw_regex_str,function_regex_str);

    /////////////////////////////////////////////////////////////////////////

    var Mode = function() {
      this.HighlightRules = N1qlHighlightRules;
      this.$completer = new N1qlCompletions();
      this.format = formatter;
    };
    oop.inherits(Mode, TextMode);

    (function() {

      this.getCompletions = function(state, session, pos, prefix) {
        return this.$completer.getCompletions(state, session, pos, prefix);
      };

      this.$id = "ace/mode/n1ql";
    }).call(Mode.prototype);

    exports.Mode = Mode;
    exports.Instance = new Mode();

  });

})();
