import SwiftUI

struct ContentView: View {
    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 16) {
                Text("Control de acceso")
                    .font(.largeTitle)
                    .fontWeight(.semibold)

                Text("Proyecto iOS inicial listo para conectar al nucleo Rust compartido.")
                    .font(.body)
                    .foregroundStyle(.secondary)

                Spacer()
            }
            .padding(24)
            .navigationTitle("Inicio")
        }
    }
}

#Preview {
    ContentView()
}
