// SPDX-License-Identifier: GPL-3.0-only
// Independent offline scatter-convolution reference. No production Rust imports,
// polyphase indexing, generator or DSP library. Compile with contraction disabled.
// Runner must pin input bytes and exact toolchain. NOT a backend or TX authority.
#include <array>
#include <cmath>
#include <complex>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

struct Spectrum { double peak_hz, image_rejection_db; std::size_t fft_frames; };
static Spectrum spectrum(const std::vector<float>& stereo, std::size_t channel) {
    const std::size_t frames=stereo.size()/2;
    std::size_t count=1;while(count<frames)count*=2;
    if(count>1048576)throw std::runtime_error("FFT capacity");
    // Independent radix-2 complex FFT, no FFTW/Rust/generator library. Analyze
    // full actual rounded samples, rectangular period then zero padding; no
    // window that could conceal a discarded tail or change image ratios.
    std::vector<std::complex<double>> a(count);
    for(std::size_t n=0;n<frames;++n)a[n]=stereo[2*n+channel];
    for(std::size_t i=1,j=0;i<count;++i) {
        std::size_t bit=count>>1;
        for(;j&bit;bit>>=1)j^=bit;
        j^=bit;if(i<j)std::swap(a[i],a[j]);
    }
    const double pi=std::acos(-1.0);
    for(std::size_t length=2;length<=count;length*=2) {
        const double angle=-2*pi/static_cast<double>(length);
        const std::complex<double> step(std::cos(angle),std::sin(angle));
        for(std::size_t i=0;i<count;i+=length) {
            std::complex<double> w(1,0);
            for(std::size_t k=0;k<length/2;++k) {
                const auto even=a[i+k],odd=a[i+k+length/2]*w;
                a[i+k]=even+odd;a[i+k+length/2]=even-odd;w*=step;
            }
        }
    }
    double main_peak=0,image_peak=0,peak_hz=0;
    for(std::size_t k=0;k<=count/2;++k) {
        const double hz=48000.0*static_cast<double>(k)/static_cast<double>(count);
        const double magnitude=std::abs(a[k]);
        if(!std::isfinite(magnitude))throw std::runtime_error("FFT finite");
        if(hz<=5200 && magnitude>main_peak){main_peak=magnitude;peak_hz=hz;}
        if(hz>=6800 && magnitude>image_peak)image_peak=magnitude;
    }
    if(main_peak<=0 || image_peak<=0)throw std::runtime_error("FFT signal/floor unavailable");
    const double rejection=20*std::log10(main_peak/image_peak);
    if(!std::isfinite(rejection)||rejection<80)throw std::runtime_error("image rejection below80dB");
    return {peak_hz,rejection,count};
}

static std::vector<char> read(const char* path, std::size_t maximum) {
    std::ifstream f(path, std::ios::binary | std::ios::ate);
    if (!f) throw std::runtime_error("missing input");
    const auto count = f.tellg();
    if (count <= 0 || static_cast<std::size_t>(count) > maximum) throw std::runtime_error("size");
    std::vector<char> b(static_cast<std::size_t>(count));
    f.seekg(0); f.read(b.data(), static_cast<std::streamsize>(b.size()));
    if (!f) throw std::runtime_error("short read");
    return b;
}
static float gain(const std::string& value) {
    if (value == "silence") return 0.0f;
    if (value.size()!=15 || value.substr(0,7)!="source:") throw std::runtime_error("explicit channel required");
    std::uint32_t bits=0;
    for (const char c:value.substr(7)) {
        const int digit = c>='0' && c<='9' ? c-'0' : c>='a' && c<='f' ? c-'a'+10 : -1;
        if (digit<0) throw std::runtime_error("gain bits");
        bits=(bits<<4)|static_cast<std::uint32_t>(digit);
    }
    float g;std::memcpy(&g,&bits,4);
    if (!std::isfinite(g)||g<=0||g>1) throw std::runtime_error("gain");
    return g;
}
int main(int argc,char** argv) {
    try {
        if(argc!=6) throw std::runtime_error("source.f32le coeff.f64le NEW-output.f32le channel0 channel1");
        const std::uint32_t endian=1;
        if(*reinterpret_cast<const unsigned char*>(&endian)!=1 || sizeof(float)!=4 || sizeof(double)!=8)
            throw std::runtime_error("qualified little-endian IEEE target required");
        auto raw=read(argv[1],720000);auto coeff=read(argv[2],4104);
        if((raw.size()!=720000 && raw.size()!=360000)||coeff.size()!=4104) throw std::runtime_error("exact geometry");
        std::vector<float> source(raw.size()/4);std::memcpy(source.data(),raw.data(),raw.size());
        std::array<double,513> h{};std::memcpy(h.data(),coeff.data(),coeff.size());
        for(double v:h)if(!std::isfinite(v))throw std::runtime_error("coefficient finite");
        const std::array<float,2> gains{gain(argv[4]),gain(argv[5])};
        if(gains[0]==0 && gains[1]==0)throw std::runtime_error("no signal channel");
        const auto frames=source.size()*4;
        // Full convolution including the region AFTER the fixed period, so a
        // cutoff at the expected output count cannot silently discard a tail.
        std::vector<double> full(frames+512,0.0);
        bool nonzero=false;
        std::size_t first=source.size(),last=0;
        for(std::size_t n=0;n<source.size();++n) {
            if(!std::isfinite(source[n]))throw std::runtime_error("source finite");
            if(source[n]!=0){if(first==source.size())first=n;last=n;nonzero=true;}
            for(std::size_t k=0;k<h.size();++k) {
                const double product=static_cast<double>(source[n])*h[k];
                full[4*n+k]+=product;
            }
        }
        if(!nonzero || last*4+513>frames)throw std::runtime_error("empty or full-tail overflow");
        for(std::size_t m=frames;m<full.size();++m)if(full[m]!=0)throw std::runtime_error("tail truncation");
        std::vector<float> output;output.reserve(frames*2);
        for(std::size_t m=0;m<frames;++m)for(float g:gains){
            float value=g==0?0.0f:static_cast<float>(full[m]*static_cast<double>(g));
            if(!std::isfinite(value)||std::abs(value)>1)throw std::runtime_error("amplitude refusal");
            output.push_back(value);
        }
        const auto measured=spectrum(output,gains[0]!=0?0:1);
        if(std::ifstream(argv[3]).good())throw std::runtime_error("output already exists");
        std::ofstream f(argv[3],std::ios::binary|std::ios::out);
        if(!f)throw std::runtime_error("output unavailable");
        f.write(reinterpret_cast<const char*>(output.data()),static_cast<std::streamsize>(output.size()*4));
        f.close();if(!f)throw std::runtime_error("output write");
        std::cout.precision(17);
        std::cout<<"frames="<<frames<<" bytes="<<output.size()*4<<" support=["<<first*4<<","<<last*4+513
            <<") fft_frames="<<measured.fft_frames<<" peak_hz="<<measured.peak_hz
            <<" image_rejection_db="<<measured.image_rejection_db<<"\n";
        return 0;
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}
