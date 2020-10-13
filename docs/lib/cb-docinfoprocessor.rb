require 'asciidoctor/extensions' unless RUBY_ENGINE == 'opal'

Asciidoctor::Extensions.register do
  if @document.basebackend? 'html'
    docinfo_processor do
      at_location :footer
      process do |doc|
        %(<script>(function(w,d,s,l,i){w[l]=w[l]||[];w[l].push({'gtm.start':
        new Date().getTime(),event:'gtm.js'});var f=d.getElementsByTagName(s)[0],
        j=d.createElement(s),dl=l!='dataLayer'?'&l='+l:'';j.async=true;j.src=
        'https://www.googletagmanager.com/gtm.js?id='+i+dl;f.parentNode.insertBefore(j,f);
        })(window,document,'script','dataLayer','GTM-MVPNN2');</script>
      <script src="https://cdn.cookielaw.org/scripttemplates/otSDKStub.js"
        type="text/javascript" charset="UTF-8" data-domain-script="748511ff-10bf-44bf-88b8-36382e5b5fd9"></script>
      <script type="text/javascript">
        function OptanonWrapper() { }
      </script>)
      end
    end
  end
end