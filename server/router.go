package server

import (
	"bytes"
	"fmt"
	"net"
	"net/http"
	"runtime/debug"
	"strings"

	"github.com/bakape/meguca/auth"
	"github.com/bakape/meguca/config"
	"github.com/bakape/meguca/db"
	"github.com/bakape/meguca/imager"
	"github.com/bakape/meguca/util"
	"github.com/dimfeld/httptreemux"
	"github.com/facebookgo/grace/gracehttp"
	"github.com/go-playground/log"

	// Add profiling to default server mux
	_ "net/http/pprof"
)

var (
	healthCheckMsg = []byte("God's in His heaven, all's right with the world")
)

// Used for overriding during tests
var webRoot = "www"

func startWebServer() (err error) {
	go func() {
		// Bind pprof to random localhost-only address
		http.ListenAndServe("localhost:0", nil)
	}()

	c := config.Server.Server

	var w strings.Builder
	w.WriteString("listening on http")
	prettyAddr := c.Address
	if len(c.Address) != 0 && c.Address[0] == ':' {
		prettyAddr = "127.0.0.1" + prettyAddr
	}
	fmt.Fprintf(&w, "://%s", prettyAddr)
	log.Info(w.String())

	gracehttp.PreStartProcess(db.Close)
	err = gracehttp.Serve(&http.Server{
		Addr:    c.Address,
		Handler: createRouter(),
	})
	if err != nil {
		return util.WrapError("error starting web server", err)
	}
	return
}

func handlePanic(w http.ResponseWriter, r *http.Request, err interface{}) {
	http.Error(w, fmt.Sprintf("500 %s", err), 500)
	ip, ipErr := auth.GetIP(r)
	if ipErr != nil {
		ip = net.IPv4zero
	}
	log.Errorf("server: %s: %#v\n%s\n", ip, err, debug.Stack())
}

// Create the monolithic router for routing HTTP requests. Separated into own
// function for easier testability.
func createRouter() http.Handler {
	r := httptreemux.NewContextMux()
	r.NotFoundHandler = func(w http.ResponseWriter, _ *http.Request) {
		text404(w)
	}
	r.PanicHandler = handlePanic

	r.GET("/robots.txt", func(w http.ResponseWriter, r *http.Request) {
		var buf bytes.Buffer
		buf.WriteString("User-agent: *\n")
		if config.Get().DisableRobots {
			buf.WriteString("Disallow: /\n")
		}
		w.Header().Set("Content-Type", "text/plain")
		buf.WriteTo(w)
	})

	api := r.NewGroup("/api")
	api.GET("/health-check", func(w http.ResponseWriter, r *http.Request) {
		w.Write(healthCheckMsg)
	})
	assets := r.NewGroup("/assets")
	if config.Server.ImagerMode != config.NoImager {
		// All upload images
		api.POST("/upload", imager.NewImageUpload)
		api.POST("/upload-hash", imager.UploadImageHash)

		assets.GET("/images/*path", serveImages)

		// // Captcha API
		// captcha := api.NewGroup("/captcha")
		// captcha.GET("/:board", serveNewCaptcha)
		// captcha.POST("/:board", authenticateCaptcha)
	}
	if config.Server.ImagerMode != config.ImagerOnly {
		// TODO: Serve index page
		// r.GET("/", )

		// JSON API
		// json := r.NewGroup("/json")
		// json.GET("/post/:post", servePost)
	}

	return r
}
