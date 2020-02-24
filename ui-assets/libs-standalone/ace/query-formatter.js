(function() {

  define("ace/mode/query-formatter",["require","exports","module","ace/lib/oop"],
      function(require, exports, module) {
    "use strict";

//    var oop = require("../lib/oop");

    //////////////////////////////////////////////////////////////////////////////////////
    // format Sql_plus_plus queries. Code borrowed and extended from vkbeautify
    //
    // Step 1: strip out newlines and extra spaces
    // Step 2: add back newlines before or after certain keywords (e.g. SELECT, WHERE...),
    //         this ensures that any nested subqueries are not on the line as their parent
    // Step 3: add further newlines in any top-level comma-delimited lists, e.g.
    //           select a, b, c from ...
    //         but *not*:
    //           select max(a,b)
    // Step 4: figure out indentation for each line. We will have a stack of indentations
    //           so that the indents will accumulate
    //         if a line increases the count of parentheses, we increase the indentation
    //         if a line ends in a comma, align subsequent lines with the first space on
    //          the first line with the comma
    //         if a line starts with a keyword needing extra indentation, indent that
    //          one line only
    //////////////////////////////////////////////////////////////////////////////////////

    exports.create = function(kw_regex_str,function_regex_str) {

      // regexes must ignore keywords inside strings or comments, make a prefix to match strings or comments
      var prefix = "\"(?:[^\"\\\\]|\\\\.)*\"|'(?:[^'\\\\]|\\\\.)*'|(?:\\/\\*[\\s\\S]*?\\*\\/)|`(?:[^`])*`";
      var match_string = new RegExp(prefix,'ig');

      // we want to detect all keywords or functions, so make a regex that matches them
      var kw_regex = new RegExp(kw_regex_str,'ig');
      var function_regex = new RegExp(function_regex_str,'ig');

      // detect keywords that need newlines before and/or after
      var newline_before = "FROM|WHERE|GROUP BY|HAVING|OUTER JOIN|INNER JOIN|JOIN|LIMIT|ORDER BY|OFFSET|OUTER JOIN|ROWS|" +
        "RANGE|GROUPS|EXCLUDE|UNNEST|SET|LET";
      var newline_before_and_after = "UNION ALL|UNION";
      var newline_before_plus_indent = "AND|OR|JOIN";
      var newline_after_regex = new RegExp("(;)","ig");

      var newline_before_regex =
        new RegExp('(?:\\bBETWEEN\\b.+?\\bAND\\b)' +    // don't want newline before AND in "BETWEEN ... AND"
            '|\\b(' + newline_before + '|' + newline_before_plus_indent + ')\\b','ig');
      var newline_before_and_after_regex = new RegExp(prefix + '|\\b(' + newline_before_and_after + ')\\b','ig');
      var newline_after_over_paren_regex = /(\bOVER[\s]*\([\s]*\))|(\bOVER[\s]*\([\s]*)/ig;
      // we need an indent if the line starts with one of these
      var needs_indent_regex = new RegExp('^(' + newline_before_plus_indent + ')\\b','ig');

      //
      //
      //

      function isSubquery(str, parenthesisLevel) {
        return  parenthesisLevel - (str.replace(/\(/g,'').length - str.replace(/\)/g,'').length )
      }

      // some text is special: comments, quoted strings, and we don't want to look inside them for formatting
      function isSpecialString(str) {
        return((str.startsWith('"') || str.startsWith("'") ||
              (str.startsWith('/*') && str.endsWith('*/')) ||
              (str.startsWith('`') && str.endsWith('`'))));
      }

      //
      // if we have commas delimiting projection lists, let statements, etc, we want a newline
      // and indentation after each comma. But we don't want newlines for comma-delimited items in function
      // calls, e.g. max(a,b,c)
      //

      function replace_top_level_commas(text,parenDepth) {
        var items = [];
        var start = 0;

        for (var i=0; i < text.length; i++) {
          switch (text.charAt(i)) {
          case '(':
          case '{':
          case '[':
            parenDepth.depth++; break;
          case ')':
          case '}':
          case ']':
            parenDepth.depth--; break;
          case ',':
            if (parenDepth.depth <= 0) {
              items.push(text.substring(start,i+1));
              while (i++ < text.length && /\s/.exec(text.charAt(i))); // skip any whitespace after the comma
              start = i;
              i--; // don't overshoot
            }
            break;
          }
        }

        // get the last one
        items.push(text.substring(start,text.length));
        return(items);
      }

      // split query into an array of lines, with strings separate entries so we won't try to
      // parse their contents
      function split_query(str, tab, specials) {

        var str2 = str.replace(/\s{1,}/g," ");                                                  // simplify whitespace to single spaces
        str2 = str2.replace(match_string, function(match) {specials.push(match); return "^&&^"});
        str2 = str2.replace(kw_regex, function(match,p1) {if (p1) return p1.toUpperCase(); else return match}); // upper case all keywords
        str2 = str2.replace(function_regex, function(match,p1) {if (p1) return p1.toUpperCase() + '('; else return match}); // upper case all keywords
        str2 = str2.replace(newline_before_regex, function(match,p1) {if (p1) return "~::~" + p1; else return match});
        str2 = str2.replace(newline_before_and_after_regex, function(match,p1) {if (p1) return "~::~" + p1 + "~::~"; else return match});
        str2 = str2.replace(newline_after_regex, function(match,p1) {if (p1) return p1 + "~::~"; else return match});
        str2 = str2.replace(newline_after_over_paren_regex, function(match,p1) {if (p1) return p1; else return 'OVER (~::~';});  // put a newline after ( in "OVER ("
        str2 = str2.replace(/\([\s]*SELECT\b/ig, function(match) {return '(~::~SELECT'});  // put a newline after ( in "( SELECT"
        str2 = str2.replace(/\)[\s]*SELECT\b/ig, function(match) {return ')~::~SELECT'});  // put a newline after ) in ") SELECT"
        str2 = str2.replace(/\)[\s]*,/ig,function(match) {return '),'});                   // remove any whitespace between ) and ,
        str2 = str2.replace(/~::~w{1,}/g,"~::~");                                          // remove blank lines
        str2 = str2.replace(/~::~ /ig,'~::~');

        // get an array of lines, based on the above breaks, then make a new array where we also split on comma-delimited lists
        var arr =  str2.split('~::~');
        var arr2 = [];

        arr.forEach(function (s) {
          var parenDepth = {depth:0};
          arr2 = arr2.concat(replace_top_level_commas(s,parenDepth));
            });

        return(arr2);
      }

      // function to format queries based on the parameters above

      var formatter = function(text, step) {
        var tab = ' '.repeat(step),
        ar,
        deep = 0,
        paren_level = 0,
        str = '',
        ix = 0,
        specials = [],
        indents = [''],  // stack of indentation strings
        parens_in_lists = []; // are nested queries part of comma-delimited lists

        ar = split_query(text,tab,specials);

        // now we have an array of either:
        // - things that should start on a newline, as indicated by starting with a special keyword
        // - things that should start on a newline, as indicated by the previous element ending with a comma
        // - strings or comments, which we can't look inside
        //
        // loop through the array of query elements.
        // we need to add appropriate indentation for each, based on this element and the previous
        // non-special element

        var comma_prev = false;
        var paren_prev = false;
        var prev_paren_level = 0;
        var inside_case = false; // are we part of a multi-line CASE statement?

        // Some will be specials, so we will just add those to the query string
        // Others need to be checked for nesting level
        var len = ar.length;

        for(ix=0;ix<len;ix++) {
          ar[ix] = ar[ix].trim();

          // remove blank lines
          if (ar[ix].length == 0)
            continue;

          // check for changes in the nesting level
          prev_paren_level = paren_level;
          paren_level = isSubquery(ar[ix], paren_level);

          // is this a string that should start or end with a new line?
          // - did the previous string end with a comma?
          // - does this string match a keyword that needs a newline?

          needs_indent_regex.lastIndex = 0;
          newline_before_and_after_regex.lastIndex = 0;
          newline_after_over_paren_regex.lastIndex = 0;
          var indent = (indents.length>0?indents[indents.length - 1]:'');

          var needs_indent = !!needs_indent_regex.exec(ar[ix]);
          var after = !!newline_before_and_after_regex.exec(ar[ix]) || !!newline_after_over_paren_regex.exec(ar[ix]);
          var ends_with_comma = ar[ix].endsWith(',');
          var ends_with_paren_comma = ar[ix].endsWith('),');
          var ends_with_paren = ar[ix].endsWith('(');

//          console.log("Got string: " + ar[ix]);
//          console.log("bfore indents len: " + indents.length + " paren_level " + paren_level +
//              " prev_paren_level " + prev_paren_level +
//              " ends_with_comma " + ends_with_comma +
//              " comma_prev " + comma_prev +
//              " parens_in_lists " + JSON.stringify(parens_in_lists)
//              );

          // each array element should start a new line, add appropriate indent
          str += '\n' + indent;

          // do we need a special indent for just this line?
          if (needs_indent)
            str += tab;

          // add the string
          str += ar[ix];

          // should there be a newline after?
          if (after)
            str += '\n';

          // if this is the first in a comma-delimited list, we need an appropriate indent.
          // if the line starts "SELECT" then we want 8 spaces indent
          // if the list starts "ORDER BY" or "GROUP BY" then we want 9 spaces indent
          // otherwise find the *last* space in the line for subsequent alignment
          if (ends_with_comma && !comma_prev && (paren_level == prev_paren_level)) {
            var fs;
            if (ar[ix].startsWith("SELECT "))
              fs = 7;
            else if (ar[ix].startsWith("GROUP BY") || ar[ix].startsWith("ORDER BY"))
              fs = 9;
            else if (ar[ix].startsWith("PARTITION BY"))
              fs = 13;
            else if (ar[ix].startsWith("LET"))
              fs = 4;
            else if (ar[ix].startsWith("INSERT INTO "))
              fs = 12;
            else for (fs = ar[ix].length - 1; fs >= 0; fs--)
              if (ar[ix].charAt(fs) == ' ') {
                fs++;
                break;
              }
            // subsequent lines should be indented by this much
            indents.push(indent + ' '.repeat(fs));
          }

          // if the nesting level goes up, add elements to the indent array,
          // if it goes down, pop them
          if (paren_level > prev_paren_level) {
            var curIndent = indent;
            for (var i=prev_paren_level; i < paren_level; i++) {
              curIndent = curIndent + tab;
              indents.push(curIndent);
            }
            parens_in_lists.push(comma_prev); // is the paren scope part of a comma list?
          }
          else if (paren_level < prev_paren_level) {
            // get rid of indentation for the parens
            for (var i=prev_paren_level; i > paren_level; i--)
              indents.pop();

            // if our paren scope had a comma list, pop that indent
            if (comma_prev)
              indents.pop();

            // go back to comma status from outside paren scope
            comma_prev = parens_in_lists.pop();
            if (comma_prev && !ends_with_comma)
              indents.pop();
          }
          // if the previous item had a comma, but this doesn't, pop the comma indent
          else if (comma_prev && !ends_with_comma)
            indents.pop();

          comma_prev = ends_with_comma; // remember comma status
          paren_prev = ends_with_paren;
        }

        // insert the special strings back into the string
        while (/\^\&\&\^/ig.exec(str)) {
          if (!specials.length)
            break;
          str = str.replace(/\^\&\&\^/ig, function(match) {return(specials.shift());});
        }

        str = str.replace(/^\n{1,}/,'').replace(/\n{1,}/g,"\n");
        return str;
      };

      return formatter;
    }


    //////////////////////////////////////////////////////////////////////////////////////

  });

})();

